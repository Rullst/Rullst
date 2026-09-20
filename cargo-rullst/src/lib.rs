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
            .subcommand(generators::age_gate::command())
            // Extend executable syntax without changing the published v12 enum.
            .mut_subcommand("omni", |command| {
                command.arg(
                    clap::Arg::new("release")
                        .long("release")
                        .action(clap::ArgAction::SetTrue)
                        .help("Build an Android release APK with application-owned signing"),
                )
            })
            .get_matches_from(args);
        if let Some(age_gate) = matches.subcommand_matches("make:age-gate") {
            generators::age_gate::run(age_gate)?;
        } else if let Some(update) = matches.subcommand_matches("update") {
            update::run(update)?;
        } else if let Some(omni) = matches
            .subcommand_matches("omni")
            .filter(|m| m.get_flag("release"))
        {
            if omni.get_one::<String>("target").map(String::as_str) != Some("android") {
                return Err("--release currently requires the android Omni target".into());
            }
            generators::desktop::build_android_release()?;
        } else {
            let cli = <cli::Cli as clap::FromArgMatches>::from_arg_matches(&matches)?;
            cli::run_cli_command(&cli.command)?;
        }
    }
    Ok(())
}
