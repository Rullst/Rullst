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
            .subcommand(generators::privacy::command())
            .subcommand(generators::supervision::command())
            .subcommand(generators::api_contract::command())
            .subcommand(generators::deploy_doctor::command())
            // Extend executable syntax without changing the published v12 enum.
            .mut_subcommand("omni", generators::desktop::release_command)
            .mut_subcommand("generate:ai-context", generators::ai_context::command)
            .get_matches_from(args);
        if let Some(doctor) = matches.subcommand_matches("deploy:doctor") {
            generators::deploy_doctor::run(doctor)?;
        } else if let Some(api) = matches.subcommand_matches("generate:api") {
            generators::api_contract::run(api)?;
        } else if let Some(context) = matches.subcommand_matches("generate:ai-context") {
            generators::ai_context::run(context)?;
        } else if let Some(supervision) = matches.subcommand_matches("make:supervision") {
            generators::supervision::run(supervision)?;
            generators::ai_context::refresh_after_scaffold();
        } else if let Some(privacy) = matches.subcommand_matches("make:privacy") {
            generators::privacy::run(privacy)?;
            generators::ai_context::refresh_after_scaffold();
        } else if let Some(age_gate) = matches.subcommand_matches("make:age-gate") {
            generators::age_gate::run(age_gate)?;
            generators::ai_context::refresh_after_scaffold();
        } else if let Some(update) = matches.subcommand_matches("update") {
            update::run(update)?;
        } else if let Some(omni) = matches
            .subcommand_matches("omni")
            .filter(|m| m.get_flag("release"))
        {
            if omni.get_one::<String>("target").map(String::as_str) != Some("android") {
                return Err("--release currently requires the android Omni target".into());
            }
            generators::desktop::run_release(omni)?;
        } else {
            let cli = <cli::Cli as clap::FromArgMatches>::from_arg_matches(&matches)?;
            cli::run_cli_command(&cli.command)?;
        }
    }
    Ok(())
}
