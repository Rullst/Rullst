//! `cargo rullst add <capability>` (v13): enables a `rullst` facade feature
//! in an existing project, documents its variables in `.env.example` and
//! prints the wiring code to paste. It never rewrites application source:
//! generated projects have no marked insertion points, so the snippet is
//! printed with where it goes. Running it twice changes nothing.

mod capability;
mod plan;
#[cfg(test)]
mod tests;

use crate::ui::error_report::{self, AlreadyReported, Friendly, ProjectRequired};
use crate::ui::style::{self, Style};
use clap::{Arg, ArgAction, ArgMatches, Command};
use plan::{AddError, AddPlan, Scope};
use std::error::Error;
use std::path::Path;

const DOCS: &str = "https://rullst.github.io/Rullst/book/cli_reference.html#cargo-rullst-add";

/// The `add` subcommand, attached in `crate::command`.
pub(crate) fn command() -> Command {
    let capabilities = capability::CAPABILITIES
        .iter()
        .map(|capability| format!("  {:<8}{}", capability.name, capability.summary))
        .collect::<Vec<_>>()
        .join("\n");
    Command::new("add")
        .about("Enables a Rullst capability (mail, auth, ai, nexus, studio) in this project")
        .long_about(format!(
            "Enables a Rullst capability in this project: turns on its `rullst` feature in \
             Cargo.toml (comments and formatting are kept), appends its variables with \
             placeholder or mock values to .env.example and prints the code to wire it in. \
             Running it again changes nothing.\n\nCapabilities:\n{capabilities}"
        ))
        .arg(
            Arg::new("capability")
                .required(true)
                .value_name("CAPABILITY")
                .value_parser(capability::NAMES)
                .help("The capability to enable"),
        )
        .arg(
            Arg::new("dry_run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Prints the planned diff without writing anything"),
        )
}

/// Next-step hints for `cargo rullst add <name>`.
pub(crate) fn hints(name: &str) -> &'static [(&'static str, &'static str)] {
    capability::find(name).map_or(&[], |capability| capability.hints)
}

/// Runs `cargo rullst add` in the current directory.
pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let name = matches
        .get_one::<String>("capability")
        .map(String::as_str)
        .unwrap_or_default();
    let dry_run = matches.get_flag("dry_run");
    let root = std::env::current_dir()?;
    let style = Style::stdout();
    match execute(&root, name, dry_run, style) {
        Ok(text) => {
            print!("{text}");
            Ok(())
        }
        Err(Failure::Project) => Err(ProjectRequired.into()),
        Err(Failure::Add(error)) => {
            eprint!(
                "{}",
                error_report::render(&friendly(&error), &[], None, false, Style::stderr())
            );
            Err(AlreadyReported::new(1, error.to_string()).into())
        }
    }
}

/// Why `execute` stopped.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    Project,
    Add(AddError),
}

/// Plans and (unless `dry_run`) applies `name` at `root`, returning the
/// report to print.
pub(crate) fn execute(
    root: &Path,
    name: &str,
    dry_run: bool,
    style: Style,
) -> Result<String, Failure> {
    let capability = capability::find(name)
        .ok_or_else(|| Failure::Add(AddError::UnknownCapability(name.to_string())))?;
    let plan = match plan::plan(root, capability).map_err(Failure::Add)? {
        Ok(plan) => plan,
        Err(Scope::NotAProject) => return Err(Failure::Project),
    };
    if !dry_run {
        plan::apply(root, &plan).map_err(Failure::Add)?;
    }
    Ok(report(&plan, dry_run, style))
}

fn friendly(error: &AddError) -> Friendly {
    let (title, fix) = match error {
        AddError::UnknownCapability(_) => (
            "Unknown capability",
            "Run `cargo rullst add --help` to list the capabilities.",
        ),
        AddError::InvalidManifest { .. } => (
            "Cargo.toml could not be read as TOML",
            "Fix the syntax at the reported line (`cargo metadata` shows the full error), then run the command again.",
        ),
        AddError::UnsupportedDependency => (
            "The rullst dependency cannot take features",
            "Declare it as `rullst = \"<version>\"` or `rullst = { version = \"<version>\", features = [...] }` under [dependencies], then run the command again.",
        ),
        AddError::Read { .. } => (
            "A project file could not be read",
            "Check the file's permissions and that it is valid UTF-8, then run the command again.",
        ),
        AddError::Write { .. } => (
            "A project file could not be written",
            "Check the file's permissions and free disk space; running the command again is safe.",
        ),
    };
    Friendly {
        title: title.to_string(),
        happened: error.to_string(),
        fix: Some(fix.to_string()),
        docs: Some(DOCS),
    }
}

fn report(plan: &AddPlan, dry_run: bool, style: Style) -> String {
    let capability = plan.capability;
    let mut out = format!(
        "{} {}\n",
        style.bold("◆", style::ACCENT),
        style.bold(
            &format!("cargo rullst add {}", capability.name),
            style::BRIGHT
        )
    );
    if plan.already_enabled {
        out.push_str(&format!(
            "  {} `{}` is already enabled in Cargo.toml.\n",
            style.paint("✓", style::PASS),
            capability.name
        ));
    }
    if plan.changes.is_empty() {
        out.push_str(&format!(
            "  {}\n",
            style.paint(
                "Nothing to change: Cargo.toml and .env.example already have it.",
                style::MUTED
            )
        ));
        return out;
    }
    for change in &plan.changes {
        let summary = if dry_run {
            format!("would {}", change.summary)
        } else {
            change.summary.clone()
        };
        out.push_str(&format!(
            "  {} {:<13} {}\n",
            style.paint(if dry_run { "~" } else { "✓" }, style::PASS),
            change.file,
            style.paint(&summary, style::MUTED)
        ));
    }
    if !plan.changes.iter().any(|change| change.file == ".env") {
        out.push_str(&format!(
            "  {} {:<13} {}\n",
            style.paint("·", style::MUTED),
            ".env",
            style.paint(
                "unchanged: nothing it lacks is needed to start in development",
                style::MUTED
            )
        ));
    }
    if dry_run {
        let diff_style = crate::ai::term::Style {
            color: !style.is_plain(),
        };
        for change in &plan.changes {
            let header = match change.before {
                Some(_) => format!("--- {0}\n+++ {0} (planned)", change.file),
                None => format!("--- /dev/null\n+++ {} (new file)", change.file),
            };
            out.push_str(&format!("\n{}\n", style.paint(&header, style::BRIGHT)));
            let (diff, _) = crate::ai::diff::render(
                change.before.as_deref().unwrap_or_default(),
                &change.after,
                diff_style,
            );
            out.push_str(&diff);
        }
    }
    if !plan.already_enabled {
        out.push_str(&format!(
            "\n{}\n  {}\n",
            style.bold("Wire it in (not edited automatically)", style::BRIGHT),
            style.paint(&format!("{}:", capability.snippet.place), style::MUTED)
        ));
        for line in capability.snippet.code.lines() {
            out.push_str(&format!("    {line}\n"));
        }
    }
    if dry_run {
        out.push_str(&format!(
            "\n{}\n",
            style.paint(
                "Dry run: nothing was written. Run the same command without --dry-run to apply it.",
                style::MUTED
            )
        ));
    }
    out
}
