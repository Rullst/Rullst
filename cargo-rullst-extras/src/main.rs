//! `cargo-rullst-extras`: optional commands for the Rullst CLI.
//!
//! `cargo rullst ai …` and `cargo rullst green …` forward here when this
//! executable is on `PATH`; it also runs directly as
//! `cargo-rullst-extras ai …` or `cargo rullst-extras ai …`.
#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unreachable,
        clippy::unwrap_used
    )
)]

mod ai;
mod green;

use clap::{Arg, ArgAction, Command};
use std::error::Error;
use std::process::ExitCode;

fn command() -> Command {
    Command::new("cargo-rullst-extras")
        .bin_name("cargo rullst")
        .version(env!("CARGO_PKG_VERSION"))
        .about(
            "Optional Rullst CLI extras: the terminal AI assistant and measured efficiency figures",
        )
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .global(true)
                .action(ArgAction::Count)
                .help("Show the underlying causes when a command fails"),
        )
        .subcommand(ai::command())
        .subcommand(green::command())
}

/// The arguments without Cargo's `rullst-extras` (from `cargo rullst-extras …`).
fn without_cargo_name(mut arguments: Vec<String>) -> Vec<String> {
    if arguments
        .get(1)
        .is_some_and(|first| first == "rullst-extras")
    {
        arguments.remove(1);
    }
    arguments
}

fn run(arguments: Vec<String>) -> Result<(), Box<dyn Error>> {
    let matches = command()
        .try_get_matches_from(without_cargo_name(arguments))
        .unwrap_or_else(|error| error.exit());
    match matches.subcommand() {
        Some(("ai", ai)) => ai::run(ai).map_err(Into::into),
        Some(("green", green)) => green::run(green),
        _ => Ok(()),
    }
}

#[cfg_attr(mutants, mutants::skip)]
fn main() -> ExitCode {
    cargo_rullst::extras::run_and_report(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_command_tree_is_valid_and_has_both_extras() {
        let command = command();
        command.clone().debug_assert();
        let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();
        assert_eq!(names, ["ai", "green"]);
        assert!(command.clone().try_get_matches_from(["x"]).is_err());
        let matches = command
            .try_get_matches_from(["x", "-v", "green", "--json"])
            .unwrap();
        assert_eq!(matches.get_count("verbose"), 1);
        assert_eq!(matches.subcommand_name(), Some("green"));
    }

    #[test]
    fn cargo_s_subcommand_name_is_dropped() {
        assert_eq!(
            without_cargo_name(strings(&["cargo-rullst-extras", "rullst-extras", "ai"])),
            ["cargo-rullst-extras", "ai"]
        );
        assert_eq!(
            without_cargo_name(strings(&["cargo-rullst-extras", "green", "rullst-extras"])),
            ["cargo-rullst-extras", "green", "rullst-extras"]
        );
        assert_eq!(without_cargo_name(strings(&["x"])), ["x"]);
    }
}
