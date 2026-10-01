//! The executable layer around the public `Commands` enum: the global
//! `-v/--verbose` flag, `--json` status views, "did you mean" for unknown
//! commands and the shared friendly error report at the process boundary.

use crate::ui::error_report::{self, AlreadyReported, Friendly, USAGE_EXIT};
use crate::ui::style::Style;
use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::error::Error;

const CLI_REFERENCE: &str = "https://rullst.github.io/Rullst/book/cli_reference.html";

fn json_flag(help: &'static str) -> Arg {
    Arg::new("json")
        .long("json")
        .action(ArgAction::SetTrue)
        .help(help)
}

/// Adds the runtime-only flags to `command`.
pub(crate) fn extend(command: Command) -> Command {
    command
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .global(true)
                .action(ArgAction::Count)
                .help("Show the underlying causes when a command fails"),
        )
        .mut_subcommand("doctor", |doctor| {
            doctor.arg(json_flag(
                "Print the checks as versioned JSON (rullst.cli-doctor.v1); exit status 1 when a check fails",
            ))
        })
}

fn flag(matches: &ArgMatches, id: &str) -> bool {
    matches
        .try_get_one::<bool>(id)
        .ok()
        .flatten()
        .copied()
        .unwrap_or(false)
}

/// The command level an unknown token was typed at, as the words leading to
/// it (`update`) and that level's command.
fn level<'a>(
    command: &'a Command,
    arguments: &[String],
    invalid: &str,
) -> (Vec<String>, &'a Command) {
    let mut current = command;
    let mut path = Vec::new();
    for argument in arguments.iter().skip(1) {
        if argument == invalid {
            break;
        }
        if argument.starts_with('-') {
            continue;
        }
        match current.find_subcommand(argument) {
            Some(subcommand) => {
                path.push(subcommand.get_name().to_string());
                current = subcommand;
            }
            None => break,
        }
    }
    (path, current)
}

/// The friendly report for an unknown command typed after `prefix`.
pub(crate) fn unknown_command(invalid: &str, prefix: &str, suggestions: &[&str]) -> Friendly {
    let fix = match suggestions {
        [] => format!(
            "Run `{prefix} --help` to list every command, or `cargo rullst` to search them."
        ),
        [only] => format!("Did you mean `{prefix} {only}`?"),
        many => {
            let lines: Vec<String> = many
                .iter()
                .map(|name| format!("  {prefix} {name}"))
                .collect();
            format!("Did you mean one of these?\n{}", lines.join("\n"))
        }
    };
    Friendly {
        title: format!("Unknown command `{invalid}`"),
        happened: format!("`{prefix}` has no `{invalid}` command."),
        fix: Some(fix),
        docs: Some(CLI_REFERENCE),
    }
}

/// Parses `arguments`; an unknown subcommand gets "did you mean" suggestions
/// on stderr and exit status 2. Help, version and other usage errors keep
/// clap's own output and exit status.
pub(crate) fn parse(
    mut command: Command,
    arguments: Vec<String>,
) -> Result<ArgMatches, Box<dyn Error>> {
    match command.try_get_matches_from_mut(&arguments) {
        Ok(matches) => Ok(matches),
        Err(error) if error.kind() == ErrorKind::InvalidSubcommand => {
            let invalid = match error.get(ContextKind::InvalidSubcommand) {
                Some(ContextValue::String(invalid)) => invalid.clone(),
                _ => error.exit(),
            };
            let (path, current) = level(&command, &arguments, &invalid);
            let names = super::suggest::command_names(current);
            let suggestions = super::suggest::suggestions(&invalid, &names);
            let prefix = std::iter::once("cargo rullst".to_string())
                .chain(path)
                .collect::<Vec<_>>()
                .join(" ");
            let report = unknown_command(&invalid, &prefix, &suggestions);
            eprint!(
                "{}",
                error_report::render(&report, &[], None, false, Style::stderr())
            );
            Err(AlreadyReported::new(USAGE_EXIT, format!("unknown command `{invalid}`")).into())
        }
        Err(error) => error.exit(),
    }
}

/// Runs the commands this layer owns (`doctor` with its `--json` view);
/// `false` when the command belongs to the regular dispatch.
pub(crate) fn run_extension(matches: &ArgMatches) -> Result<bool, Box<dyn Error>> {
    match matches.subcommand() {
        Some(("doctor", sub)) => {
            crate::generators::doctor::run(crate::generators::doctor::DoctorOptions {
                fix: flag(sub, "fix"),
                json: flag(sub, "json"),
            })?
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// The binaries' entry point: runs the CLI and renders any failure with the
/// shared friendly report. Returns the process exit status.
#[doc(hidden)]
pub fn run_and_report() -> std::process::ExitCode {
    let arguments: Vec<String> = std::env::args_os()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    error_report::install_panic_hook(error_report::verbose_requested(&arguments));
    if Style::stdout().is_plain() {
        // Generators colour through `colored`; keep pipes, CI and NO_COLOR plain.
        colored::control::set_override(false);
    }
    match crate::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            std::process::ExitCode::from(error_report::report(error.as_ref(), &arguments))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(list: &[&str]) -> Vec<String> {
        list.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn the_runtime_tree_is_valid_and_doctor_accepts_json() {
        let command = crate::command();
        command.clone().debug_assert();
        assert!(command.find_subcommand("doctor").is_some());
        let matches = command
            .try_get_matches_from(["rullst", "doctor", "--json", "-v"])
            .expect("doctor --json -v parses");
        let (_, doctor) = matches.subcommand().expect("doctor");
        assert!(flag(doctor, "json"));
        assert!(!flag(doctor, "fix"));
    }

    #[test]
    fn unknown_commands_suggest_the_closest_names_with_their_prefix() {
        let command = crate::command();
        let args = arguments(&["rullst", "make:modek", "Post"]);
        let (path, top) = level(&command, &args, "make:modek");
        assert!(path.is_empty());
        let names = super::super::suggest::command_names(top);
        let suggestions = super::super::suggest::suggestions("make:modek", &names);
        assert_eq!(suggestions, ["make:model"]);
        let report = unknown_command("make:modek", "cargo rullst", &suggestions);
        assert_eq!(report.title, "Unknown command `make:modek`");
        assert_eq!(
            report.fix.as_deref(),
            Some("Did you mean `cargo rullst make:model`?")
        );

        let args = arguments(&["rullst", "update", "chekk"]);
        let (path, update) = level(&command, &args, "chekk");
        assert_eq!(path, ["update"]);
        let names = super::super::suggest::command_names(update);
        assert_eq!(
            super::super::suggest::suggestions("chekk", &names),
            ["check"]
        );

        let none = unknown_command("zzz", "cargo rullst", &[]);
        assert!(none.fix.unwrap_or_default().contains("cargo rullst --help"));
        let many = unknown_command("db", "cargo rullst", &["db:seed", "db:status"]);
        assert_eq!(
            many.fix.as_deref(),
            Some("Did you mean one of these?\n  cargo rullst db:seed\n  cargo rullst db:status")
        );
    }

    #[test]
    fn parsing_an_unknown_command_fails_with_the_usage_status() {
        let error =
            parse(crate::command(), arguments(&["rullst", "doctr"])).expect_err("unknown command");
        assert_eq!(error.to_string(), "unknown command `doctr`");
        let error: &(dyn Error + 'static) = error.as_ref();
        assert_eq!(error_report::report(error, &[]), USAGE_EXIT);
    }
}
