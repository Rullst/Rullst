#![allow(
    clippy::collapsible_if,
    clippy::unnecessary_map_or,
    clippy::redundant_async_block
)]
extern crate self as rullst;

pub mod config;
pub mod db;
pub mod edge;

#[cfg(not(target_arch = "wasm32"))]
pub mod ai;
#[cfg(not(target_arch = "wasm32"))]
pub mod artisan;
#[cfg(not(target_arch = "wasm32"))]
pub mod auth;
#[cfg(not(target_arch = "wasm32"))]
pub mod capital;
#[cfg(not(target_arch = "wasm32"))]
pub mod cache;
pub mod client;
#[cfg(not(target_arch = "wasm32"))]
pub mod error_console;
#[cfg(not(target_arch = "wasm32"))]
pub mod feature;
#[cfg(not(target_arch = "wasm32"))]
pub mod horizon;
pub mod html;
#[cfg(not(target_arch = "wasm32"))]
pub mod htmx;
#[cfg(not(target_arch = "wasm32"))]
pub mod live;
#[cfg(not(target_arch = "wasm32"))]
pub mod mail;
#[cfg(not(target_arch = "wasm32"))]
pub mod multitenant;
#[cfg(not(target_arch = "wasm32"))]
pub mod queue;
#[cfg(not(target_arch = "wasm32"))]
pub mod resilience;
#[cfg(not(target_arch = "wasm32"))]
pub mod routing;
#[cfg(not(target_arch = "wasm32"))]
pub mod scheduler;
#[cfg(not(target_arch = "wasm32"))]
pub mod security;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
#[cfg(not(target_arch = "wasm32"))]
pub mod storage;
#[cfg(not(target_arch = "wasm32"))]
pub mod studio;
#[cfg(not(target_arch = "wasm32"))]
pub mod nexus;
#[cfg(not(target_arch = "wasm32"))]
pub mod testing;
#[cfg(not(target_arch = "wasm32"))]
pub mod validation;
#[cfg(not(target_arch = "wasm32"))]
pub mod ws;

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! artisan {
    ($migrations:expr) => {
        let _ = $crate::artisan::check_and_run_artisan($migrations, vec![]).await;
    };
    ($migrations:expr, $seeders:expr) => {
        let _ = $crate::artisan::check_and_run_artisan($migrations, $seeders).await;
    };
}

// Re-export the html! and client_component procedural macros
pub use rullst_macros::{client_component, html};

// Re-export core structs for public consumption
#[cfg(not(target_arch = "wasm32"))]
pub use routing::Router;

#[cfg(not(target_arch = "wasm32"))]
pub use server::Server;

// Re-export rullst-orm for seamless database usage
#[cfg(not(target_arch = "wasm32"))]
pub use rullst_orm::{RullstModel, Orm};

// Re-export Configuration types
pub use config::{AppConfig, DatabaseConfig, RullstConfig, SecurityConfig};

// Re-export axum response types for convenience
#[cfg(not(target_arch = "wasm32"))]
pub mod response {
    pub use axum::response::{Html, IntoResponse, Redirect, Response};
}

// Re-export HTMX primitives for convenience
#[cfg(not(target_arch = "wasm32"))]
pub use htmx::{HtmxRequest, HtmxResponse, render_page};

// Re-export Milestone 5: Production Utilities
#[cfg(not(target_arch = "wasm32"))]
pub use cache::Cache;
#[cfg(not(target_arch = "wasm32"))]
pub use queue::{Queue, QueuedJobDetail, Worker};
#[cfg(not(target_arch = "wasm32"))]
pub use scheduler::Scheduler;

// Re-export Milestone 6: Enterprise Features
#[cfg(not(target_arch = "wasm32"))]
pub use mail::{Mail, Message as MailMessage};
#[cfg(not(target_arch = "wasm32"))]
pub use storage::{Storage, StorageDriver, StorageError};
#[cfg(not(target_arch = "wasm32"))]
pub use validation::{Validate, ValidatedForm, ValidatedJson, ValidationError};
#[cfg(not(target_arch = "wasm32"))]
pub use ws::{WebSocket, WsError};

// Re-export Milestone 6 Resilience Features
#[cfg(not(target_arch = "wasm32"))]
pub use resilience::{
    RateLimitConfig, RateLimiter, TrafficShield, TrafficShieldConfig, backpressure_middleware,
    rate_limit_middleware,
};

#[cfg(not(target_arch = "wasm32"))]
pub use ai::{
    AiClient, AiError, AiProvider, ChatBuilder, Message as AiMessage, VectorDocument, VectorIndex,
};
#[cfg(not(target_arch = "wasm32"))]
pub use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))]
pub use feature::{
    DbFeatureDriver, EnvFeatureDriver, FeatureDriver, FeatureManager, MemoryFeatureDriver,
    TomlFeatureDriver,
};
#[cfg(not(target_arch = "wasm32"))]
pub use multitenant::{TenantConfig, TenantLayer, TenantService, TenantStrategy, tenant_layer};

#[cfg(not(target_arch = "wasm32"))]
pub use testing::{TestApp, TestRequestBuilder, TestResponse};

// Re-export Milestone 9: Nexus Panel (Auto-Generated CMS & AI Admin)
#[cfg(not(target_arch = "wasm32"))]
pub use nexus::{FieldKind, FieldMeta, Nexus, NexusModel};

// Re-export Milestone 9: Rullst Capital (Billing Boilerplate)
#[cfg(not(target_arch = "wasm32"))]
pub use capital::{BillingProvider, LemonSqueezyProvider, StripeProvider, SubscriptionStatus, WebhookEvent};

// ─── Dependency Shielding cascades (Roadmap Milestone 8) ────────────────────

pub mod web {
    #[cfg(not(target_arch = "wasm32"))]
    pub use axum;
    #[cfg(not(target_arch = "wasm32"))]
    pub use tower;
    #[cfg(not(target_arch = "wasm32"))]
    pub use tower_http;
}

pub mod async_runtime {
    #[cfg(not(target_arch = "wasm32"))]
    pub use tokio;
}

pub mod email_client {
    #[cfg(feature = "mail-smtp")]
    #[cfg(not(target_arch = "wasm32"))]
    pub use lettre;
}
