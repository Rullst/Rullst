//! Interactive dev error console with source context inspection and AI assistance.

pub mod api;
pub mod middleware;
pub mod parser;
pub mod renderer;

#[cfg(test)]
mod tests;

pub use api::*;
pub use middleware::*;
pub use parser::*;
pub(crate) use renderer::*;
