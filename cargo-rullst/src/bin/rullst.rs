#![deny(
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::unwrap_used
)]

use std::process::ExitCode;

/// Returning the error from `main` would print its `Debug` form; users need
/// the `Display` message instead.
#[cfg_attr(mutants, mutants::skip)]
fn main() -> ExitCode {
    match cargo_rullst::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}
