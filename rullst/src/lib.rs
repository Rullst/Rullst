pub mod ai;
pub mod artisan;
pub mod auth;
pub mod cache;
pub mod error_console;
pub mod feature;
pub mod horizon;
pub mod html;
pub mod htmx;
pub mod mail;
pub mod queue;
pub mod routing;
pub mod scheduler;
pub mod security;
pub mod server;
pub mod storage;
pub mod testing;
pub mod validation;
pub mod ws;

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
    pub use axum::response::{Html, IntoResponse, Redirect, Response};
}

// Re-export HTMX primitives for convenience
pub use htmx::{HtmxRequest, HtmxResponse, render_page};

// Re-export Milestone 5: Production Utilities
pub use cache::Cache;
pub use queue::{Queue, QueuedJobDetail, Worker};
pub use scheduler::Scheduler;

// Re-export Milestone 6: Enterprise Features
pub use mail::{Mail, Message as MailMessage};
pub use storage::{Storage, StorageDriver, StorageError};
pub use validation::{Validate, ValidatedForm, ValidatedJson, ValidationError};
pub use ws::{WebSocket, WsError};

pub use ai::{
    AiClient, AiError, AiProvider, ChatBuilder, Message as AiMessage, VectorDocument, VectorIndex,
};
pub use async_trait::async_trait;
pub use feature::{
    DbFeatureDriver, EnvFeatureDriver, FeatureDriver, FeatureManager, MemoryFeatureDriver,
    TomlFeatureDriver,
};
pub use testing::{TestApp, TestRequestBuilder, TestResponse};
