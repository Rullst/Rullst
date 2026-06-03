#![allow(
    clippy::needless_borrows_for_generic_args,
    clippy::manual_strip,
    clippy::collapsible_if
)]

pub mod blueprints;
pub mod cli;
pub mod docs_generator;
pub mod generators;
pub mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ui::trigger_background_update_check();

    let mut args: Vec<String> = std::env::args().collect();

    // Cargo passes the subcommand name ("rullst") as the first argument to the binary.
    // We remove it so clap can parse the actual commands correctly.
    if args.len() >= 2 && args[1] == "rullst" {
        args.remove(1);
    }

    if args.len() == 1 {
        ui::show_interactive_dashboard()?;
    } else {
        let cli = <cli::Cli as clap::Parser>::parse_from(args);
        cli::run_cli_command(&cli.command)?;
    }
    Ok(())
}
