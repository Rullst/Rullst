//! Interactive dev error console with source context inspection and a
//! `cargo rullst ai fix <error-id>` hand-off to the terminal assistant.

pub mod api;
pub(crate) mod capture;
pub mod middleware;
pub mod parser;
pub mod renderer;
pub(crate) mod store;

#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod tests;

pub use api::*;
pub use middleware::*;
pub use parser::*;
pub(crate) use renderer::*;
