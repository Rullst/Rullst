//! Interactive dev error console with source context inspection and an
//! "Ask your AI assistant" panel: a prompt to copy into the developer's own
//! assistant and, where the server records errors, the path of the recorded
//! context (`GET /_rullst/errors/{id}`).

pub mod api;
pub(crate) mod assistant;
pub(crate) mod capture;
pub mod middleware;
pub mod parser;
pub mod renderer;
pub(crate) mod store;

#[cfg(test)]
mod assistant_tests;
#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod tests;

pub use api::*;
pub use middleware::*;
pub use parser::*;
pub(crate) use renderer::*;
