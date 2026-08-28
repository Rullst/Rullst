// src/generators/build/upgrade.rs — Transactional, reviewable project upgrades.

mod backup;
mod manifest;
mod scan;

use crate::ui::spinner::with_spinner;
use colored::Colorize;
use manifest::ManifestUpgradePlan;
use scan::SourceFinding;
use semver::Version;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    if !root.join("Cargo.toml").is_file() {
        return Err(UpgradeError::NotRullstProject.into());
    }
    let target = target_version(options.target.as_deref())?;
    let plans = manifest::plan_workspace(&root, &target.to_string())?;
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

    let findings = scan::scan_workspace(&package_roots, &source_majors, target.major)?;
    let json_report = render_json_report(&root, &target, &plans, &findings)?;
    if options.json {
        println!("{json_report}");
    } else {
        print_plan(&root, &target, &plans, &findings, options.dry_run);
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
        &render_report(&root, &target, &plans, &findings),
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
            recovery,
        }
        .into());
    }

    let check_ok = with_spinner("Validating the migrated feature selection...", || {
        cargo_command(&root, &["check", "--workspace", "--all-targets"])
    });
    if !check_ok {
        let recovery = recover_after_failure(&backup, options.keep_on_failure)?;
        return Err(UpgradeError::CommandFailed {
            command: "cargo check --workspace --all-targets",
            recovery,
        }
        .into());
    }

    println!(
        "{}",
        format!(
            "\nUpgrade transaction completed and cargo check passed.\nBackup and review report: {}\nRun the full application tests, database restore/migration rehearsal, and deployment smoke tests before merging.",
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

fn print_plan(
    root: &Path,
    target: &Version,
    plans: &[ManifestUpgradePlan],
    findings: &[SourceFinding],
    dry_run: bool,
) {
    let mode = if dry_run { "DRY RUN" } else { "APPLY" };
    println!(
        "{}",
        format!("\nRullst assisted upgrade — {mode} — target {target}")
            .cyan()
            .bold()
    );
    println!("Project: {}", root.display());

    for plan in plans {
        let relative = plan.path.strip_prefix(root).unwrap_or(&plan.path);
        for change in &plan.changes {
            println!(
                "  {}: {} ({}) {} -> {}",
                relative.display(),
                change.key,
                change.package,
                change.from,
                change.to
            );
        }
        for warning in &plan.warnings {
            println!("  REVIEW {}: {}", relative.display(), warning);
        }
    }

    for finding in findings {
        let relative = finding.path.strip_prefix(root).unwrap_or(&finding.path);
        println!(
            "  {} {}:{} [{}] {}",
            finding.severity,
            relative.display(),
            finding.line,
            finding.code,
            finding.message
        );
    }
}

fn render_report(
    root: &Path,
    target: &Version,
    plans: &[ManifestUpgradePlan],
    findings: &[SourceFinding],
) -> String {
    let mut report = format!(
        "# Rullst assisted upgrade report\n\n- Project: `{}`\n- Target: `{target}`\n- Scope: dependency manifests, Cargo.lock and compiler-provided Rust fixes\n\n",
        root.display()
    );
    report.push_str("## Dependency plan\n\n");
    for plan in plans {
        let relative = plan.path.strip_prefix(root).unwrap_or(&plan.path);
        for change in &plan.changes {
            report.push_str(&format!(
                "- `{}`: `{}` (`{}`) `{}` → `{}`\n",
                relative.display(),
                change.key,
                change.package,
                change.from,
                change.to
            ));
        }
        for warning in &plan.warnings {
            report.push_str(&format!("- REVIEW `{}`: {}\n", relative.display(), warning));
        }
    }
    report.push_str("\n## Source review\n\n");
    if findings.is_empty() {
        report.push_str(
            "No applicable source markers from the current rule catalog were detected. This is not proof of runtime compatibility.\n",
        );
    } else {
        for finding in findings {
            let relative = finding.path.strip_prefix(root).unwrap_or(&finding.path);
            report.push_str(&format!(
                "- **{}** `{}` line {} (`{}`): {}\n",
                finding.severity,
                relative.display(),
                finding.line,
                finding.code,
                finding.message
            ));
        }
    }
    report.push_str(
        "\n## Mandatory manual gates\n\n- Review every diff and the v5 → v12 migration guide.\n- Restore a database backup into a disposable environment and rehearse migrations and rollback.\n- Run formatting, Clippy, the complete application tests, authorization negatives and a production-profile smoke test.\n- Revalidate Nexus, Studio, providers, proxy trust, CSRF/CORS and secrets.\n",
    );
    report
}

fn render_json_report(
    root: &Path,
    target: &Version,
    plans: &[ManifestUpgradePlan],
    findings: &[SourceFinding],
) -> Result<String, serde_json::Error> {
    let manifests = plans
        .iter()
        .map(|plan| {
            serde_json::json!({
                "path": plan.path.strip_prefix(root).unwrap_or(&plan.path),
                "matched_dependencies": plan.matched,
                "source_majors": plan.source_majors,
                "changes": plan.changes,
                "warnings": plan.warnings,
            })
        })
        .collect::<Vec<_>>();
    let findings = findings
        .iter()
        .map(|finding| {
            serde_json::json!({
                "path": finding.path.strip_prefix(root).unwrap_or(&finding.path),
                "line": finding.line,
                "code": finding.code,
                "severity": finding.severity,
                "message": finding.message,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&serde_json::json!({
        "schema_version": "rullst.upgrade-plan.v1",
        "rule_catalog": scan::RULE_CATALOG_VERSION,
        "target": target.to_string(),
        "manifests": manifests,
        "source_findings": findings,
        "automatic_scope": [
            "workspace dependency manifests",
            "Cargo.lock resolution",
            "compiler-provided Rust fixes",
            "cargo check for the selected features"
        ],
        "manual_gates": [
            "review the complete diff",
            "rehearse database restore, migration and rollback",
            "run the full application tests and authorization negatives",
            "validate providers, proxy trust, Nexus, Studio, CSRF/CORS and secrets"
        ],
        "production_ready": false
    }))
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
    fn target_accepts_an_exact_prerelease_in_the_cli_train() {
        let cli = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        let prerelease = format!("{}.1.0-rc.2", cli.major);
        assert_eq!(
            target_version(Some(&prerelease)).unwrap().to_string(),
            prerelease
        );
    }
}
