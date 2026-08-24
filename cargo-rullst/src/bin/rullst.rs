#![deny(
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::unwrap_used
)]

#[cfg_attr(mutants, mutants::skip)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    cargo_rullst::run()
}
