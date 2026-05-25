pub mod html;
pub mod routing;
pub mod server;
pub mod artisan;
pub mod auth;
pub mod security;
pub mod htmx;

#[macro_export]
macro_rules! artisan {
    ($migrations:expr) => {
        let _ = $crate::artisan::check_and_run_artisan($migrations, vec![]).await;
    };
    ($migrations:expr, $seeders:expr) => {
        let _ = $crate::artisan::check_and_run_artisan($migrations, $seeders).await;
    };
}

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

// Re-export HTMX primitives for convenience
pub use htmx::{HtmxRequest, HtmxResponse, render_page};

