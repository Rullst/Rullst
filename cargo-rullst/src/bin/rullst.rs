#![deny(
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::unwrap_used
)]

use std::process::ExitCode;

/// Failures are rendered by the CLI's shared friendly error report (title,
/// what happened, how to fix, docs) instead of a `Debug` dump.
#[cfg_attr(mutants, mutants::skip)]
fn main() -> ExitCode {
    cargo_rullst::cli::run_and_report()
}
