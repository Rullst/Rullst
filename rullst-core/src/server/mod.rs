//! Application server bootstrap, hot-reloader, and Tower middleware integration.

/// Fluent server builder and HTTP runner.
pub mod builder;
pub(crate) mod console;
pub(crate) mod database_url;
mod dev_reload;
/// Dynamic library router loader for hot-reload mode.
pub mod dylib_loader;
/// Atomic hot-swappable Tower service.
pub mod hotswap;
mod scheduler_supervision;
/// Server-level HTTP middlewares (HMR script injection, static asset compression).
pub mod server_middleware;
mod stack;
mod traffic;

#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
mod tests;

pub use crate::Router;
#[doc(hidden)]
pub use builder::read_optional_environment_variable;
pub use builder::{Server, ServerError};
#[cfg(feature = "orm")]
#[doc(hidden)]
pub use database_url::resolve_project_database_url;
pub use hotswap::HotSwapService;
pub use server_middleware::{inject_hmr_script, zstd_static_middleware};

// ─── Dependency Shielding cascades (Roadmap Milestone 8) ────────────────────
pub use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Extension, Form, Json, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header},
    middleware::{self, Next, from_fn},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, patch, post, put},
};
