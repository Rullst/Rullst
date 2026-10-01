// src/cli/mod.rs — Clap command definitions and the central dispatch function.
#![cfg_attr(mutants, mutants::skip)]
// This is the nerve center of the CLI: defines every subcommand and routes
// each one to its corresponding generator function. The value enums, the
// subcommand enum and the dispatcher live in focused submodules and are
// re-exported here so every public item keeps its `cargo_rullst::cli` path.

use clap::Parser;

mod choices;
mod commands;
pub(crate) mod completions;
mod dispatch;
pub(crate) mod info;
pub(crate) mod runtime;
pub(crate) mod suggest;
#[cfg(test)]
mod tests;

pub use choices::{BlueprintChoice, DatabaseChoice, OmniPlatformChoice};
pub use commands::Commands;
pub use dispatch::run_cli_command;
#[doc(hidden)]
pub use runtime::run_and_report;

// ─── Clap Structs ─────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "rullst")]
#[command(version)]
#[command(about = "Official CLI for the Rullst Framework", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
