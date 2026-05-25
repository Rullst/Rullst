pub mod html;
pub mod routing;
pub mod server;
pub mod artisan;
pub mod auth;
pub mod security;
pub mod htmx;
pub mod queue;
pub mod cache;
pub mod scheduler;
pub mod validation;
pub mod mail;
pub mod storage;
pub mod ws;
pub mod horizon;
pub mod ai;
pub mod testing;
pub mod feature;
pub mod error_console;

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

// Re-export Milestone 5: Production Utilities
pub use queue::{Queue, Worker, QueuedJobDetail};
pub use cache::Cache;
pub use scheduler::Scheduler;

// Re-export Milestone 6: Enterprise Features
pub use validation::{ValidatedForm, ValidatedJson, Validate, ValidationError};
pub use mail::{Mail, Message as MailMessage};
pub use storage::{Storage, StorageDriver, StorageError};
pub use ws::{WebSocket, WsError};

pub use ai::{AiClient, AiProvider, AiError, Message as AiMessage, ChatBuilder, VectorIndex, VectorDocument};
pub use testing::{TestApp, TestRequestBuilder, TestResponse};
pub use feature::{FeatureManager, FeatureDriver, MemoryFeatureDriver, EnvFeatureDriver, TomlFeatureDriver, DbFeatureDriver};
pub use async_trait::async_trait;
