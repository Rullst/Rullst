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

    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        ui::show_interactive_dashboard()?;
    } else {
        let cli = <cli::Cli as clap::Parser>::parse();
        cli::run_cli_command(&cli.command)?;
    }
    Ok(())
}
