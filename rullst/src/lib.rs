//! Rullst Core Library
#![allow(
    clippy::collapsible_if,
    clippy::unnecessary_map_or,
    clippy::redundant_async_block
)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![warn(missing_docs)]
extern crate self as rullst;

/// Rullst application and server configuration models.
pub mod config;
pub mod db;
pub mod edge;

#[cfg(not(target_arch = "wasm32"))]
/// AI integrations (e.g. Chat, Vector indices, Embeddings).
pub mod ai;
#[cfg(not(target_arch = "wasm32"))]
/// Artisan command-line migrations and seed execution helpers.
pub mod artisan;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(all(not(target_arch = "wasm32"), feature = "auth"))]
/// Complete authentication system supporting session, JWT, and Passkey.
pub mod auth;
#[cfg(not(target_arch = "wasm32"))]
pub mod cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod capital;
/// Edge client components rendering module.
pub mod client;
#[cfg(not(target_arch = "wasm32"))]
/// HTML visual logging and runtime console for development mode.
pub mod error_console;
#[cfg(not(target_arch = "wasm32"))]
/// Feature flagging management and drivers.
pub mod feature;
#[cfg(not(target_arch = "wasm32"))]
/// Background job queues monitoring panel (Horizon).
pub mod horizon;
/// Fast compile-time HTML rendering utilities.
pub mod html;
#[cfg(not(target_arch = "wasm32"))]
/// HTMX helpers for rapid reactive UI design.
pub mod htmx;
#[cfg(not(target_arch = "wasm32"))]
/// Live state synchronization and server-push connection handlers.
pub mod live;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(all(not(target_arch = "wasm32"), feature = "mailer"))]
/// Unified mail delivery and dispatch drivers.
pub mod mail;
#[cfg(not(target_arch = "wasm32"))]
/// Multitenancy request routing and tenant state isolation layers.
pub mod multitenant;
#[cfg(all(not(target_arch = "wasm32"), feature = "nexus"))]
pub mod nexus;
#[cfg(not(target_arch = "wasm32"))]
pub mod queue;
#[cfg(not(target_arch = "wasm32"))]
/// Network and service resilience (rate limits, traffic shield, load shedding).
pub mod resilience;
#[cfg(not(target_arch = "wasm32"))]
/// High-performance application routers built on Axum.
pub mod routing;
#[cfg(not(target_arch = "wasm32"))]
pub mod scheduler;
#[cfg(not(target_arch = "wasm32"))]
/// CSRF tokens, CORS, and response headers security policies.
pub mod security;
#[cfg(not(target_arch = "wasm32"))]
/// Rullst HTTP web server core engine.
pub mod server;
#[cfg(not(target_arch = "wasm32"))]
/// File and cloud storage abstraction layer.
pub mod storage;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(all(not(target_arch = "wasm32"), feature = "studio"))]
/// Developer's Studio dashboard for database, logs, and queue analytics.
pub mod studio;
#[cfg(not(target_arch = "wasm32"))]
/// Rullst OpenTelemetry and Tracing.
pub mod telemetry;
#[cfg(not(target_arch = "wasm32"))]
/// Scaffolding tools for robust integration testing.
pub mod testing;
#[cfg(not(target_arch = "wasm32"))]
/// Strongly-typed request validation extractors.
pub mod validation;
#[cfg(not(target_arch = "wasm32"))]
/// WebSocket live connection and messaging system.
pub mod ws;

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
/// Standard migration and seeding bootstrap macro for Rullst.
macro_rules! artisan {
    ($migrations:expr) => {
        let _ = $crate::artisan::check_and_run_artisan($migrations, vec![]).await;
    };
    ($migrations:expr, $seeders:expr) => {
        let _ = $crate::artisan::check_and_run_artisan($migrations, $seeders).await;
    };
}

// Re-export procedural macros
pub use rullst_macros::{html, island, live_component, live_event, memoize, route, server_function};

// Re-export core structs for public consumption
#[cfg(not(target_arch = "wasm32"))]
pub use routing::Router;

#[cfg(not(target_arch = "wasm32"))]
pub use server::Server;

// Re-export rullst-orm for seamless database usage
#[cfg(not(target_arch = "wasm32"))]
pub use rullst_orm::{Orm, RullstModel};

// Re-export Configuration types
pub use config::{AppConfig, DatabaseConfig, RullstConfig, SecurityConfig};

// Re-export axum response types for convenience
#[cfg(not(target_arch = "wasm32"))]
/// Standard HTTP response types and helpers.
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
#[cfg(all(not(target_arch = "wasm32"), feature = "mailer"))]
pub mod mail_exports {
    pub use crate::mail::{Mail, Message as MailMessage};
}
#[cfg(all(not(target_arch = "wasm32"), feature = "mailer"))]
pub use mail_exports::*;
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
#[cfg(all(not(target_arch = "wasm32"), feature = "nexus"))]
pub mod nexus_exports {
    pub use crate::nexus::{FieldKind, FieldMeta, Nexus, NexusModel};
}
#[cfg(all(not(target_arch = "wasm32"), feature = "nexus"))]
pub use nexus_exports::*;

// Re-export Milestone 9: Rullst Capital (Billing Boilerplate)
#[cfg(not(target_arch = "wasm32"))]
pub use capital::{
    BillingProvider, LemonSqueezyProvider, StripeProvider, SubscriptionStatus, WebhookEvent,
};

// ─── Dependency Shielding cascades (Roadmap Milestone 8) ────────────────────

#[cfg(not(target_arch = "wasm32"))]
/// Internal asynchronous runtime wrappers.
pub mod runtime {
    pub use async_trait::async_trait;
    pub use tokio::{main, spawn, task, time};
}

/// Web server engine re-exports (Axum, Tower, Tower-HTTP).
pub mod web {
    #[cfg(not(target_arch = "wasm32"))]
    pub use axum;
    #[cfg(not(target_arch = "wasm32"))]
    pub use tower;
    #[cfg(not(target_arch = "wasm32"))]
    pub use tower_http;
}

/// Re-exported underlying asynchronous engine (Tokio).
pub mod async_runtime {
    #[cfg(not(target_arch = "wasm32"))]
    pub use tokio;
}

/// Re-exported email client engine (Lettre).
pub mod email_client {
    #[cfg(feature = "mail-smtp")]
    #[cfg(not(target_arch = "wasm32"))]
    pub use lettre;
}
