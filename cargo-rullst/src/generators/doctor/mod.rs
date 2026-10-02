//! `cargo rullst doctor`: grouped health checks for the toolchain, the
//! project, its configuration, database reachability, migrations, the
//! security baseline and disk space. Each problem carries a one-line fix and
//! a docs link; `--json` prints the versioned `rullst.cli-doctor.v1` report
//! and `--fix` installs missing rustfmt/clippy components. The command exits
//! with status 1 when any check fails (warnings do not fail it).

mod context;
mod database;
mod disk;
pub(crate) mod probe;
mod project;
mod render;
mod report;
mod toolchain;

use crate::ui::error_report::AlreadyReported;
use crate::ui::style::Style;
use context::{ProcessVars, ProjectContext, Vars};
use report::{Check, Group, Report};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use toolchain::Probes;

/// How the doctor runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DoctorOptions {
    /// Install missing rustfmt/clippy components with rustup.
    pub fix: bool,
    /// Print the versioned JSON report instead of text.
    pub json: bool,
}

/// Runs the doctor with human output; `auto_fix` installs missing
/// rustfmt/clippy components. Fails when any check fails.
pub fn run_doctor(auto_fix: bool) -> Result<(), Box<dyn std::error::Error>> {
    run(DoctorOptions {
        fix: auto_fix,
        json: false,
    })
}

/// The outcome of `--fix` for the rustfmt/clippy components.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixOutcome {
    NotAttempted,
    Fixed,
    Failed,
}

/// Installs the components; JSON mode captures rustup's output so stdout
/// stays a single JSON document.
fn install_components(json: bool) -> bool {
    let mut command = Command::new("rustup");
    command
        .args(["component", "add", "rustfmt", "clippy"])
        .stdin(Stdio::null());
    if json {
        command.output().is_ok_and(|output| output.status.success())
    } else {
        command.status().is_ok_and(|status| status.success())
    }
}

fn components_after_fix(outcome: FixOutcome, probes: &Probes) -> Option<Check> {
    match outcome {
        FixOutcome::NotAttempted => None,
        FixOutcome::Fixed => Some(Check::pass(
            "toolchain.components",
            "rustfmt & clippy",
            "installed by --fix",
        )),
        FixOutcome::Failed => {
            let mut check = toolchain::components_check(&probes.rustfmt, &probes.clippy);
            check.detail = format!("--fix could not install them; {}", check.detail);
            check.fix =
                Some("Install them manually: rustup component add rustfmt clippy".to_string());
            Some(check)
        }
    }
}

/// Builds the report from probes and the project around `cwd`.
fn collect(
    cwd: &Path,
    project: Option<&ProjectContext>,
    probes: &Probes,
    vars: &impl Vars,
    fix: FixOutcome,
) -> Report {
    let mut toolchain = toolchain::checks(probes);
    if let Some(replacement) = components_after_fix(fix, probes) {
        for check in &mut toolchain {
            if check.id == replacement.id {
                *check = replacement.clone();
            }
        }
    }
    let mut groups = vec![Group::new("toolchain", "Toolchain", "toolchain", toolchain)];
    let audit = toolchain::cargo_audit_check(&probes.cargo_audit);
    match project {
        Some(project) => {
            groups.push(Group::new(
                "project",
                "Project",
                "project",
                project::project_checks(project),
            ));
            groups.push(Group::new(
                "config",
                "Config & .env",
                "config-and-env",
                project::config_checks(project, vars),
            ));
            groups.push(Group::new(
                "database",
                "Database",
                "database",
                database::database_checks(project, vars),
            ));
            groups.push(Group::new(
                "migrations",
                "Migrations",
                "migrations",
                database::migration_checks(project, vars),
            ));
            let mut security = vec![audit];
            security.extend(project::env_git_check(&project.root, probes.git.ok()));
            security.extend(project::app_key_check(project, vars));
            security.push(project::lockfile_check(&project.root));
            groups.push(Group::new(
                "security",
                "Security baseline",
                "security-baseline",
                security,
            ));
        }
        None => {
            groups.push(Group::new(
                "project",
                "Project",
                "project",
                project::outside_checks(),
            ));
            groups.push(Group::new(
                "security",
                "Security baseline",
                "security-baseline",
                vec![audit],
            ));
        }
    }
    let (disk_path, shown) = match project {
        Some(project) => (
            project.root.as_path(),
            project
                .relative_root
                .clone()
                .unwrap_or_else(|| ".".to_string()),
        ),
        None => (cwd, ".".to_string()),
    };
    groups.push(Group::new(
        "disk",
        "Disk",
        "disk-space",
        disk::checks(disk_path, &shown),
    ));
    Report::new(groups)
}

/// Runs the doctor. With `--json` stdout carries exactly one JSON document.
pub(crate) fn run(options: DoctorOptions) -> Result<(), Box<dyn std::error::Error>> {
    let style = if options.json {
        Style::PLAIN
    } else {
        Style::stdout()
    };
    let mut stdout = io::stdout();
    if !options.json {
        write!(stdout, "{}", render::header(style))?;
        stdout.flush()?;
    }
    let cwd = std::env::current_dir()?;
    let project = ProjectContext::detect(&cwd);
    let needs_wasm = project
        .as_ref()
        .is_some_and(|project| project.root.join("src").join("islands").is_dir());
    let mut probes = Probes::collect(needs_wasm);
    let mut outcome = FixOutcome::NotAttempted;
    if options.fix && !(probes.rustfmt.ok() && probes.clippy.ok()) {
        if !options.json {
            writeln!(
                stdout,
                "Fixing rustfmt & clippy: rustup component add rustfmt clippy\n"
            )?;
            stdout.flush()?;
        }
        let installed = install_components(options.json);
        probes.rustfmt = probe::run("cargo", &["fmt", "--version"]);
        probes.clippy = probe::run("cargo", &["clippy", "--version"]);
        outcome = if installed && probes.rustfmt.ok() && probes.clippy.ok() {
            FixOutcome::Fixed
        } else {
            FixOutcome::Failed
        };
        if !options.json {
            writeln!(stdout)?;
        }
    }
    let report = collect(&cwd, project.as_ref(), &probes, &ProcessVars, outcome);
    if options.json {
        writeln!(stdout, "{}", serde_json::to_string_pretty(&report)?)?;
    } else {
        write!(stdout, "{}", render::render(&report, style))?;
    }
    stdout.flush()?;
    if !report.ok {
        return Err(AlreadyReported::new(
            1,
            format!(
                "cargo rullst doctor found {} failing check(s)",
                report.summary.fail
            ),
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
