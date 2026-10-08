//! The executable layer around the public `Commands` enum: the global
//! `-v/--verbose` flag, runtime-only subcommands (`completions`, `info`),
//! `--json` status views, "did you mean" for unknown commands and the shared
//! friendly error report at the process boundary.

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

/// Adds the runtime-only flags and subcommands to `command`.
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
        .subcommand(super::completions::command())
        .subcommand(super::info::command())
        .mut_subcommand("doctor", |doctor| {
            doctor.arg(json_flag(
                "Print the checks as versioned JSON (rullst.cli-doctor.v1); exit status 1 when a check fails",
            ))
        })
        .mut_subcommand("audit", |audit| {
            audit
                .arg(json_flag(
                    "Print a versioned JSON summary (rullst.cli-audit.v1); progress moves to stderr",
                ))
                .arg(
                    Arg::new("report")
                        .long("report")
                        .value_name("FORMAT")
                        .num_args(0..=1)
                        .default_missing_value("md")
                        .value_parser(["md", "html", "json"])
                        .conflicts_with_all(["json", "compliance", "sbom", "network", "geiger", "ai"])
                        .help("Write an evidence report mapped to OWASP ASVS 5.0 Level 1 (md, html or json; default md) to SECURITY_REPORT.<format>; exit status 1 on FINDINGS or ERROR"),
                )
                .arg(
                    Arg::new("output")
                        .long("output")
                        .value_name("PATH")
                        .value_parser(clap::value_parser!(std::path::PathBuf))
                        .requires("report")
                        .help("Write the --report file to PATH instead of the project root"),
                )
        })
        .mut_subcommand("inspect", |inspect| {
            inspect.arg(json_flag(
                "Print `inspect routes` as versioned JSON (rullst.cli-routes.v1)",
            ))
        })
}

/// The `audit --report` format, when the flag was given.
fn report_format(matches: &ArgMatches) -> Option<crate::generators::audit_report::ReportFormat> {
    matches
        .try_get_one::<String>("report")
        .ok()
        .flatten()
        .and_then(|value| crate::generators::audit_report::ReportFormat::parse(value))
}

fn audit_ignores(matches: &ArgMatches) -> Vec<String> {
    matches
        .try_get_many::<String>("audit_ignore")
        .ok()
        .flatten()
        .map(|values| values.cloned().collect())
        .unwrap_or_default()
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

/// The command-line arguments as text. A non-Unicode argument (for example a
/// Latin-1 file name) gets a usage report and exit status 2 instead of a
/// panic; the program path itself is converted lossily.
pub(crate) fn unicode_arguments(
    arguments: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut converted = Vec::new();
    for (index, argument) in arguments.into_iter().enumerate() {
        match argument.into_string() {
            Ok(argument) => converted.push(argument),
            Err(program) if index == 0 => converted.push(program.to_string_lossy().into_owned()),
            Err(argument) => {
                let report = Friendly {
                    title: "Invalid argument".to_string(),
                    happened: format!(
                        "Argument {index} (`{}`) is not valid Unicode.",
                        argument.to_string_lossy()
                    ),
                    fix: Some(
                        "cargo rullst accepts UTF-8 arguments only: rename the file or pass the value in UTF-8."
                            .to_string(),
                    ),
                    docs: Some(CLI_REFERENCE),
                };
                eprint!(
                    "{}",
                    error_report::render(&report, &[], None, false, Style::stderr())
                );
                return Err(AlreadyReported::new(
                    USAGE_EXIT,
                    format!("argument {index} is not valid Unicode"),
                )
                .into());
            }
        }
    }
    Ok(converted)
}

/// Runs `completions`, `info` and the `--json` views; `false` when the
/// command belongs to the regular dispatch.
pub(crate) fn run_extension(matches: &ArgMatches) -> Result<bool, Box<dyn Error>> {
    match matches.subcommand() {
        Some(("completions", sub)) => super::completions::run(sub)?,
        Some(("info", sub)) => super::info::run(sub)?,
        Some(("doctor", sub)) => {
            crate::generators::doctor::run(crate::generators::doctor::DoctorOptions {
                fix: flag(sub, "fix"),
                json: flag(sub, "json"),
            })?
        }
        Some(("audit", sub)) if report_format(sub).is_some() => {
            let ignores = audit_ignores(sub);
            crate::generators::audit_report::run_report(
                crate::generators::audit_report::ReportOptions {
                    format: report_format(sub)
                        .unwrap_or(crate::generators::audit_report::ReportFormat::Markdown),
                    output: sub
                        .try_get_one::<std::path::PathBuf>("output")
                        .ok()
                        .flatten()
                        .cloned(),
                    ignores: &ignores,
                },
            )?;
        }
        Some(("audit", sub)) if flag(sub, "json") => {
            let ignores = audit_ignores(sub);
            crate::generators::audit::run_audit(crate::generators::audit::AuditOptions {
                ai: flag(sub, "ai"),
                compliance: flag(sub, "compliance"),
                idor: flag(sub, "idor"),
                geiger: flag(sub, "geiger"),
                sbom: flag(sub, "sbom"),
                ignores: &ignores,
                network: flag(sub, "network"),
                json: true,
            })?;
        }
        Some(("inspect", sub)) if flag(sub, "json") => {
            let target = sub
                .try_get_one::<String>("target")
                .ok()
                .flatten()
                .map_or("routes", String::as_str);
            crate::generators::inspect::print_json(target)?;
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
    error_report::install_panic_hook(error_report::panic_details_requested(&arguments));
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
    fn the_runtime_tree_is_valid_and_has_the_new_entry_points() {
        let command = crate::command();
        command.clone().debug_assert();
        for name in [
            "completions",
            "info",
            "version",
            "doctor",
            "update",
            "deploy:doctor",
        ] {
            assert!(command.find_subcommand(name).is_some(), "{name}");
        }
        let matches = command
            .clone()
            .try_get_matches_from(["rullst", "doctor", "--json", "-v"])
            .expect("doctor --json -v parses");
        let (_, doctor) = matches.subcommand().expect("doctor");
        assert!(flag(doctor, "json"));
        assert!(!flag(doctor, "fix"));
        assert!(
            command
                .clone()
                .try_get_matches_from(["rullst", "inspect", "routes", "--json"])
                .is_ok()
        );
        let audit = command
            .try_get_matches_from([
                "rullst",
                "audit",
                "--json",
                "--audit-ignore",
                "RUSTSEC-2099-0001",
            ])
            .expect("audit --json parses");
        let (_, audit) = audit.subcommand().expect("audit");
        assert!(flag(audit, "json"));
        assert_eq!(
            audit
                .try_get_many::<String>("audit_ignore")
                .ok()
                .flatten()
                .map(|values| values.cloned().collect::<Vec<_>>()),
            Some(vec!["RUSTSEC-2099-0001".to_string()])
        );
    }

    #[test]
    fn audit_report_parses_its_format_output_and_conflicts() {
        let command = crate::command();
        let parse = |arguments: &[&str]| command.clone().try_get_matches_from(arguments);
        let matches = parse(&["rullst", "audit", "--report"]).expect("bare --report");
        let (_, audit) = matches.subcommand().expect("audit");
        assert_eq!(
            report_format(audit),
            Some(crate::generators::audit_report::ReportFormat::Markdown)
        );
        let matches = parse(&[
            "rullst",
            "audit",
            "--report",
            "json",
            "--output",
            "out/report.json",
            "--audit-ignore",
            "RUSTSEC-2099-0001",
        ])
        .expect("--report json --output");
        let (_, audit) = matches.subcommand().expect("audit");
        assert_eq!(
            report_format(audit),
            Some(crate::generators::audit_report::ReportFormat::Json)
        );
        assert_eq!(audit_ignores(audit), ["RUSTSEC-2099-0001"]);
        assert!(parse(&["rullst", "audit", "--report", "pdf"]).is_err());
        assert!(parse(&["rullst", "audit", "--report", "--json"]).is_err());
        assert!(parse(&["rullst", "audit", "--report", "html", "--compliance"]).is_err());
        assert!(parse(&["rullst", "audit", "--output", "x.md"]).is_err());
        let plain = parse(&["rullst", "audit", "--json"]).expect("audit --json");
        assert_eq!(report_format(plain.subcommand().expect("audit").1), None);
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

    #[cfg(unix)]
    #[test]
    fn non_unicode_arguments_are_a_usage_error_not_a_panic() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let latin1 = OsString::from_vec(b"caf\xe9.rs".to_vec());
        let error = unicode_arguments([
            OsString::from("rullst"),
            OsString::from("inspect"),
            latin1.clone(),
        ])
        .expect_err("a non-Unicode argument is refused");
        assert_eq!(error.to_string(), "argument 2 is not valid Unicode");
        let error: &(dyn Error + 'static) = error.as_ref();
        assert_eq!(error_report::report(error, &[]), USAGE_EXIT);

        // The program path may be anything; it is only displayed.
        let program = unicode_arguments([latin1, OsString::from("info")]).expect("program path");
        assert_eq!(program, ["caf\u{fffd}.rs", "info"]);
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
