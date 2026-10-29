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
#[doc(hidden)]
pub mod diff;
#[doc(hidden)]
pub mod extras;
pub mod generators;
pub mod pkg;
#[doc(hidden)]
pub mod term;
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

    // `ai` and `green` belong to cargo-rullst-extras: everything after the
    // subcommand reaches it unparsed, `--help` included.
    if let Some(subcommand) = args.get(1).filter(|name| cli::forward::is_extras(name)) {
        return cli::forward::run(subcommand, args.get(2..).unwrap_or_default());
    }

    if args.len() == 1 {
        ui::trigger_background_update_check();
        ui::show_interactive_dashboard()?;
    } else {
        // Unknown commands get "did you mean" suggestions and exit status 2.
        let matches = cli::runtime::parse(command(), args)?;
        if cli::runtime::run_extension(&matches)? {
            return Ok(());
        }
        if let Some(add) = matches.subcommand_matches("add") {
            generators::add::run(add)?;
        } else if let Some((name, extras)) = matches
            .subcommand()
            .filter(|(name, _)| cli::forward::is_extras(name))
        {
            // Reached only when a global flag precedes the subcommand.
            let arguments: Vec<String> = extras
                .get_many::<String>("arguments")
                .map(|values| values.cloned().collect())
                .unwrap_or_default();
            return cli::forward::run(name, &arguments);
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
    }
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
        .subcommands(cli::forward::commands())
        .subcommand(tour::command())
        // Extend executable syntax without changing the published v12 enum.
        .mut_subcommand("generate:ai-context", generators::ai_context::command)
        .mut_subcommand("new", generators::project::new_command);
    cli::runtime::extend(command)
}
