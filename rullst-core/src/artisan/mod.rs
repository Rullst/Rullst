//! Database Artisan CLI execution and developer Studio dashboard server.

// Its items are re-exported below; `artisan::runner::*` is a second path.
#[doc(hidden)]
pub mod runner;
pub(crate) mod studio_server;
pub(crate) mod studio_views;

#[cfg(test)]
mod dotenv_tests;
#[cfg(test)]
mod tests;

pub use runner::*;
