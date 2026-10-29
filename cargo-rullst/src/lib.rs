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
mod diff;
pub mod generators;
pub mod pkg;
mod term;
mod tour;
pub mod ui;
mod update;

#[cfg_attr(mutants, mutants::skip)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // `std::env::args` panics on a non-Unicode argument; this reports it.
    let mut args = cli::runtime::unicode_arguments(std::env::args_os())?;

    // Cargo passes the subcommand name ("rullst") as the first argument to the binary.
    // When invoked directly as `rullst cli ...`, we also accept "cli" gracefully.
    if args.len() >= 2 && (args[1] == "rullst" || args[1] == "cli") {
        args.remove(1);
    }

    // Removed commands report their replacement before parsing, so that
    // their former arguments (`--help` included) reach the same report.
    if let Some(name) = args.get(1).filter(|name| cli::removed::is_removed(name)) {
        return Err(cli::removed::report(name));
    }

    // At most once a day, in an interactive terminal: announces a newer
    // cargo-rullst with its install command. Never installs anything.
    ui::trigger_background_update_check();

    if args.len() == 1 {
        ui::show_interactive_dashboard()?;
    } else {
        run_command(args)?;
        ui::update_check::print_pending_notice();
    }
    Ok(())
}

/// Parses and runs one command (everything except the home screen).
#[cfg_attr(mutants, mutants::skip)]
fn run_command(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Unknown commands get "did you mean" suggestions and exit status 2.
    let matches = cli::runtime::parse(command(), args)?;
    if cli::runtime::run_extension(&matches)? {
        return Ok(());
    }
    if let Some(add) = matches.subcommand_matches("add") {
        generators::add::run(add)?;
    } else if let Some(name) = matches
        .subcommand_name()
        .filter(|name| cli::removed::is_removed(name))
    {
        // Reached only when a global flag precedes the subcommand.
        return Err(cli::removed::report(name));
    } else if let Some(green) = matches.subcommand_matches("green") {
        generators::green::run(green)?;
    } else if let Some(doctor) = matches.subcommand_matches("deploy:doctor") {
        generators::deploy_doctor::run(doctor)?;
    } else if let Some(api) = matches.subcommand_matches("generate:api") {
        generators::api_contract::run(api)?;
    } else if let Some(context) = matches.subcommand_matches("generate:ai-context") {
        generators::ai_context::run(context)?;
    } else if let Some(privacy) = matches.subcommand_matches("make:privacy") {
        generators::privacy::run(privacy)?;
        generators::ai_context::refresh_after_scaffold();
    } else if let Some(age_gate) = matches.subcommand_matches("make:age-gate") {
        generators::age_gate::run(age_gate)?;
        generators::ai_context::refresh_after_scaffold();
    } else if let Some(pwa) = matches.subcommand_matches("make:pwa") {
        generators::pwa::run(pwa)?;
    } else if let Some(oauth) = matches.subcommand_matches("make:oauth") {
        generators::oauth::run(oauth)?;
        generators::ai_context::refresh_after_scaffold();
    } else if let Some(update) = matches.subcommand_matches("update") {
        update::run(update)?;
    } else if let Some(tour) = matches.subcommand_matches("tour") {
        tour::run(tour)?;
    } else if let Some(new) = matches.subcommand_matches("new") {
        let cli = <cli::Cli as clap::FromArgMatches>::from_arg_matches(&matches)?;
        generators::project::run_new_command(&cli.command, new)?;
    } else {
        let cli = <cli::Cli as clap::FromArgMatches>::from_arg_matches(&matches)?;
        cli::run_cli_command(&cli.command)?;
    }
    cli::next_steps::print_after_success(&matches);
    Ok(())
}

/// The complete executable command tree, shared by parsing, the command
/// palette and shell completions.
pub(crate) fn command() -> clap::Command {
    // Extend executable commands without adding a variant to the v12 public
    // Commands enum, which downstream Rust callers may exhaustively match.
    let command = <cli::Cli as clap::CommandFactory>::command()
        .subcommand(update::command())
        .subcommand(generators::add::command())
        .subcommand(generators::age_gate::command())
        .subcommand(generators::privacy::command())
        .subcommand(generators::pwa::command())
        .subcommand(generators::oauth::command())
        .subcommand(generators::api_contract::command())
        .subcommand(generators::deploy_doctor::command())
        .subcommand(generators::green::command())
        .subcommands(cli::removed::commands())
        .subcommand(tour::command())
        // Extend executable syntax without changing the published v12 enum.
        .mut_subcommand("generate:ai-context", generators::ai_context::command)
        .mut_subcommand("new", generators::project::new_command);
    cli::runtime::extend(command)
}
