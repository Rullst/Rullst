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

pub mod blueprints;
pub mod cli;
pub mod generators;
pub mod pkg;
pub mod ui;
mod update;

#[cfg_attr(mutants, mutants::skip)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().collect();

    // Cargo passes the subcommand name ("rullst") as the first argument to the binary.
    // When invoked directly as `rullst cli ...`, we also accept "cli" gracefully.
    if args.len() >= 2 && (args[1] == "rullst" || args[1] == "cli") {
        args.remove(1);
    }

    if args.len() == 1 {
        ui::trigger_background_update_check();
        ui::show_interactive_dashboard()?;
    } else {
        // Extend executable commands without adding a variant to the v12 public
        // Commands enum, which downstream Rust callers may exhaustively match.
        let matches = <cli::Cli as clap::CommandFactory>::command()
            .subcommand(update::command())
            .get_matches_from(args);
        if let Some(update) = matches.subcommand_matches("update") {
            update::run(update)?;
        } else {
            let cli = <cli::Cli as clap::FromArgMatches>::from_arg_matches(&matches)?;
            cli::run_cli_command(&cli.command)?;
        }
    }
    Ok(())
}
