pub mod html;
pub mod routing;
pub mod server;

// Re-export the html! procedural macro
pub use rullst_macros::html;

// Re-export core structs for public consumption
pub use routing::Router;

pub use server::Server;

// Re-export rust-eloquent for seamless database usage
pub use rust_eloquent::{Eloquent, EloquentModel};

// Re-export axum response types for convenience
pub mod response {
    pub use axum::response::{Html, IntoResponse, Response, Redirect};
}
