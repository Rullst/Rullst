// src/generators/build/upgrade.rs — Transactional, reviewable project upgrades.

mod backup;
mod isolated;
mod manifest;
mod report;
mod rules;
mod scan;

pub(crate) use isolated::{
    prepare_manifests, validate_prepared_manifests, validate_prepared_resolution,
};

use crate::ui::spinner::with_spinner;
use colored::Colorize;
use manifest::ManifestUpgradePlan;
use rules::{FindingKind, SourceScan};
use semver::Version;
use std::path::{Path, PathBuf};
use std::process::Command;

/// v13 upgrades v12 and v13 projects; the `rullst` crate never shipped v11, and
/// v5/v6 (with their v11-era ecosystem crates) upgrade with the v12 CLI first.
const OLDEST_SUPPORTED_SOURCE_MAJOR: u64 = 12;

fn portable_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn relative_report_path(root: &Path, path: &Path) -> String {
    portable_path(path.strip_prefix(root).unwrap_or(path))
}

#[derive(Debug, Clone, Default)]
pub struct UpgradeOptions {
    pub target: Option<String>,
    pub dry_run: bool,
    pub json: bool,
    pub keep_on_failure: bool,
    pub restore: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
enum UpgradeError {
    #[error("this command must be executed at the root of a Rullst project")]
    NotRullstProject,
    #[error("invalid target version `{0}`; use an exact SemVer version")]
    InvalidTargetVersion(String),
    #[error(
        "target {target} is incompatible with cargo-rullst {cli}; install the CLI from the same major release train"
    )]
    IncompatibleCli { target: Version, cli: Version },
    #[error("no Rullst dependencies were found in the Cargo workspace manifests")]
    NoManagedDependencies,
    #[error("{command} failed; {recovery}")]
    CommandFailed {
        command: &'static str,
        recovery: String,
    },
    #[error(
        "this project depends on Rullst {0}; this CLI upgrades from v12 or later. Upgrade to v12 first with `cargo install cargo-rullst --version '^12' --locked` and `cargo rullst upgrade`, then rerun this CLI"
    )]
    RetiredSourceMajor(String),
    #[error(
        "upgrading to {target} would downgrade `{package}` ({current}); install a cargo-rullst release that is not older than the project, or pass `--to` with such a version"
    )]
    Downgrade {
        package: String,
        current: String,
        target: Version,
    },
}

/// A validated plan: dependency edits and source findings, nothing written.
struct Planned {
    target: Version,
    plans: Vec<ManifestUpgradePlan>,
    scan: SourceScan,
    changed: usize,
}

fn plan_project(
    root: &Path,
    requested: Option<&str>,
) -> Result<Planned, Box<dyn std::error::Error>> {
    if !root.join("Cargo.toml").is_file() {
        return Err(UpgradeError::NotRullstProject.into());
    }
    let target = target_version(requested)?;
    let plans = manifest::plan_workspace(root, &target.to_string())?;
    let matched = plans.iter().map(|plan| plan.matched).sum::<usize>();
    let changed = plans.iter().map(|plan| plan.changes.len()).sum::<usize>();
    let source_majors = plans
        .iter()
        .flat_map(|plan| plan.source_majors.iter().copied())
        .collect::<std::collections::BTreeSet<_>>();
    let mut package_roots = plans
        .iter()
        .filter(|plan| plan.is_package)
        .filter_map(|plan| plan.path.parent().map(Path::to_path_buf))
        .collect::<Vec<_>>();
    package_roots.sort();
    package_roots.dedup();

    if matched == 0 {
        return Err(UpgradeError::NoManagedDependencies.into());
    }
    let retired = source_majors
        .iter()
        .filter(|major| **major < OLDEST_SUPPORTED_SOURCE_MAJOR)
        .map(|major| format!("v{major}"))
        .collect::<Vec<_>>();
    if !retired.is_empty() {
        return Err(UpgradeError::RetiredSourceMajor(retired.join(", ")).into());
    }

    reject_downgrade(root, &target, &plans)?;
    scan::reject_symlinked_sources(&package_roots)?;
    let scan = rules::scan_workspace(root, &plans, target.major)?;
    Ok(Planned {
        target,
        plans,
        scan,
        changed,
    })
}

/// Recovery advice when a Cargo gate fails while must-change findings remain.
fn findings_hint(scan: &SourceScan) -> String {
    match scan.count(FindingKind::MustChange) {
        0 => String::new(),
        count => format!(
            "; the plan lists {count} must-change finding(s): fix them first, or rerun with --keep-on-failure to repair the kept state"
        ),
    }
}

pub fn run_upgrade(options: UpgradeOptions) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?.canonicalize()?;
    if let Some(backup_path) = options.restore.as_deref() {
        let restored = backup::UpgradeBackup::restore_from(&root, backup_path)?;
        println!(
            "{}",
            format!(
                "Restored the Rullst upgrade backup from {}. Review the working tree before continuing.",
                restored.display()
            )
            .green()
            .bold()
        );
        return Ok(());
    }
    let Planned {
        target,
        plans,
        scan,
        changed,
    } = plan_project(&root, options.target.as_deref())?;
    let json_report = report::render_json_report(&root, &target, &plans, &scan)?;
    if options.json {
        println!("{json_report}");
    } else {
        report::print_plan(&root, &target, &plans, &scan, options.dry_run);
    }

    if options.dry_run {
        if !options.json {
            println!(
                "{}",
                "\nDry run complete: no files or lockfile were changed."
                    .green()
                    .bold()
            );
        }
        return Ok(());
    }

    if changed == 0 {
        println!(
            "{}",
            "\nNo version edits are required. Review any path/git warnings while the current graph is validated."
                .cyan()
        );
    }

    let backup = backup::UpgradeBackup::create(&root, &plans)?;
    let report_path = backup.write_reports(
        &report::render_report(&root, &target, &plans, &scan),
        &json_report,
    )?;

    if let Err(error) = manifest::apply_plans(&plans) {
        let recovery = recover_after_failure(&backup, options.keep_on_failure)?;
        return Err(UpgradeError::CommandFailed {
            command: "writing Cargo.toml",
            recovery: format!("{recovery}; original error: {error}"),
        }
        .into());
    }

    let fix_ok = with_spinner("Applying compiler-provided migrations...", || {
        cargo_command(
            &root,
            &[
                "fix",
                "--workspace",
                "--all-targets",
                "--allow-no-vcs",
                "--allow-dirty",
            ],
        )
    });
    if !fix_ok {
        let recovery = recover_after_failure(&backup, options.keep_on_failure)?;
        return Err(UpgradeError::CommandFailed {
            command: "cargo fix --workspace --all-targets",
            recovery: format!("{recovery}{}", findings_hint(&scan)),
        }
        .into());
    }

    let check_ok = with_spinner("Validating the migrated feature selection...", || {
        cargo_command(
            &root,
            &["check", "--workspace", "--all-targets", "--locked"],
        )
    });
    if !check_ok {
        let recovery = recover_after_failure(&backup, options.keep_on_failure)?;
        return Err(UpgradeError::CommandFailed {
            command: "cargo check --workspace --all-targets --locked",
            recovery: format!("{recovery}{}", findings_hint(&scan)),
        }
        .into());
    }

    let remaining = if scan.findings.is_empty() {
        String::new()
    } else {
        format!(
            "\n{} must-change and {} review finding(s) are listed in the report.",
            scan.count(FindingKind::MustChange),
            scan.count(FindingKind::Review),
        )
    };
    println!(
        "{}",
        format!(
            "\nUpgrade transaction completed and cargo check passed.\nBackup and review report: {}{remaining}\nRun the full application tests, database restore/migration rehearsal, and deployment smoke tests before merging.",
            report_path.display()
        )
        .green()
        .bold()
    );
    Ok(())
}

fn target_version(requested: Option<&str>) -> Result<Version, UpgradeError> {
    let cli_text = env!("CARGO_PKG_VERSION");
    let cli = Version::parse(cli_text)
        .map_err(|_| UpgradeError::InvalidTargetVersion(cli_text.to_string()))?;
    let target_text = requested.unwrap_or(cli_text);
    let target = Version::parse(target_text)
        .map_err(|_| UpgradeError::InvalidTargetVersion(target_text.to_string()))?;

    if target.major != cli.major {
        return Err(UpgradeError::IncompatibleCli { target, cli });
    }
    Ok(target)
}

/// Rejects a plan that would pin a requirement, or move `Cargo.lock`, below
/// `target`, as an older CLI (or `--to`) would otherwise do silently.
fn reject_downgrade(
    root: &Path,
    target: &Version,
    plans: &[ManifestUpgradePlan],
) -> Result<(), Box<dyn std::error::Error>> {
    for change in plans.iter().flat_map(|plan| &plan.changes) {
        if requirement_exceeds(&change.from, target) {
            return Err(UpgradeError::Downgrade {
                package: change.package.clone(),
                current: format!("requirement `{}`", change.from),
                target: target.clone(),
            }
            .into());
        }
    }
    if let Some((package, version)) = isolated::locked_above(root, target)? {
        return Err(UpgradeError::Downgrade {
            package,
            current: format!("{version} in Cargo.lock"),
            target: target.clone(),
        }
        .into());
    }
    Ok(())
}

/// Whether any lower bound of `requirement` admits only versions above `target`.
fn requirement_exceeds(requirement: &str, target: &Version) -> bool {
    let Ok(requirement) = semver::VersionReq::parse(requirement) else {
        return false;
    };
    requirement.comparators.iter().any(|comparator| {
        let mut minimum = Version::new(
            comparator.major,
            comparator.minor.unwrap_or(0),
            comparator.patch.unwrap_or(0),
        );
        minimum.pre = comparator.pre.clone();
        match comparator.op {
            semver::Op::Greater => minimum >= *target,
            semver::Op::Exact
            | semver::Op::GreaterEq
            | semver::Op::Tilde
            | semver::Op::Caret
            | semver::Op::Wildcard => minimum > *target,
            _ => false,
        }
    })
}

fn cargo_command(root: &Path, args: &[&str]) -> bool {
    Command::new("cargo")
        .args(args)
        .current_dir(root)
        .status()
        .is_ok_and(|status| status.success())
}

fn recover_after_failure(
    backup: &backup::UpgradeBackup,
    keep_on_failure: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    if keep_on_failure {
        Ok(format!(
            "edited files were kept by request; recover with `cargo rullst upgrade --restore {}`",
            backup.root().display()
        ))
    } else {
        backup.restore()?;
        Ok(format!(
            "the original manifests, lockfile, and Rust sources were restored; diagnostic report: {}; the persisted snapshot can be restored again with `cargo rullst upgrade --restore {}`",
            backup.report_path().display(),
            backup.root().display()
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn target_must_match_the_installed_cli_major() {
        let cli = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        let incompatible = format!("{}.0.0", cli.major + 1);
        assert!(matches!(
            target_version(Some(&incompatible)),
            Err(UpgradeError::IncompatibleCli { .. })
        ));
    }

    #[test]
    fn requirements_above_the_target_are_downgrades() {
        let target = Version::parse("13.0.0").unwrap();
        for newer in [
            "=13.2.0", "13.1", "^13.0.1", "~13.0.5", ">=14", ">13.0.0", "13.1.*",
        ] {
            assert!(requirement_exceeds(newer, &target), "{newer}");
        }
        for not_newer in [
            "=13.0.0",
            "13",
            "12",
            ">=12, <14",
            "<13.5",
            "*",
            "13.0.0-rc.1",
        ] {
            assert!(!requirement_exceeds(not_newer, &target), "{not_newer}");
        }
        // A stable requirement is above a prerelease CLI of the same version.
        let prerelease = Version::parse("13.0.0-alpha.1").unwrap();
        assert!(requirement_exceeds("13", &prerelease));
        assert!(!requirement_exceeds("13.0.0-alpha.1", &prerelease));
    }

    #[test]
    fn target_accepts_an_exact_prerelease_in_the_cli_train() {
        let cli = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        let prerelease = format!("{}.1.0-rc.2", cli.major);
        assert_eq!(
            target_version(Some(&prerelease)).unwrap().to_string(),
            prerelease
        );
    }
}
