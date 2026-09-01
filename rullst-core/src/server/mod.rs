//! Application server bootstrap, hot-reloader, and Tower middleware integration.

/// Fluent server builder and HTTP runner.
pub mod builder;
/// Dynamic library router loader for hot-reload mode.
pub mod dylib_loader;
/// Atomic hot-swappable Tower service.
pub mod hotswap;
/// Server-level HTTP middlewares (HMR script injection, static asset compression).
pub mod server_middleware;

#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
mod tests;

pub use crate::Router;
pub use builder::{Server, ServerError};
pub use hotswap::HotSwapService;
pub use server_middleware::{inject_hmr_script, zstd_static_middleware};

// ─── Dependency Shielding cascades (Roadmap Milestone 8) ────────────────────
pub use axum::{
    body::{Body, Bytes},
    extract::{Extension, Form, Json, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header},
    middleware::{self, Next, from_fn},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, patch, post, put},
};
