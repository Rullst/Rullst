//! Studio Control Center & Data Browser Module

#![cfg_attr(mutants, mutants::skip)]

pub mod db;
pub mod handlers;
pub mod layout;

#[cfg(test)]
mod tests;

pub use db::{TableQuery, ensure_pool_initialized, resolve_db_url};
pub use handlers::*;
pub use layout::{render_sidebar_oob, studio_layout};

use axum::{Router, extract::DefaultBodyLimit};

/// Central router for Rullst Studio
pub fn router() -> Router {
    router_with_trace_store(crate::distributed_traces::DistributedTraceStore::default())
}

/// Builds the Studio router against an explicitly supplied distributed trace store.
pub fn router_with_trace_store(
    trace_store: crate::distributed_traces::DistributedTraceStore,
) -> Router {
    Router::new()
        // Dashboard
        .route("/", axum::routing::get(handle_dashboard))
        .route("/studio", axum::routing::get(handle_dashboard))
        // Tables Data Browser
        // rullst-access: admin — composed behind LocalStudioAccess::protect_router.
        .route("/tables/{table}", axum::routing::get(handle_table))
        // rullst-access: admin — composed behind LocalStudioAccess::protect_router.
        .route("/studio/tables/{table}", axum::routing::get(handle_table))
        // These handlers additionally require the request-local proof inserted
        // by LocalStudioAccess after its loopback and same-origin checks.
        .route(
            "/studio/tables/{table}/rows/update",
            axum::routing::post(handle_table_update).layer(DefaultBodyLimit::max(64 * 1024)),
        )
        .route(
            "/studio/tables/{table}/rows/delete",
            axum::routing::post(handle_table_delete).layer(DefaultBodyLimit::max(64 * 1024)),
        )
        // Core Studio Navigation Routes
        .route(
            "/migrations",
            axum::routing::get(handle_studio_tools_migrations),
        )
        .route(
            "/studio/migrations",
            axum::routing::get(handle_studio_tools_migrations),
        )
        .route("/ai", axum::routing::get(handle_studio_tools_ai))
        .route("/studio/ai", axum::routing::get(handle_studio_tools_ai))
        .route(
            "/security",
            axum::routing::get(handle_studio_tools_security),
        )
        .route(
            "/studio/security",
            axum::routing::get(handle_studio_tools_security),
        )
        .route("/radar", axum::routing::get(handle_studio_radar))
        .route("/studio/radar", axum::routing::get(handle_studio_radar))
        .route("/capital", axum::routing::get(handle_studio_capital))
        .route("/studio/capital", axum::routing::get(handle_studio_capital))
        .route(
            "/traces",
            axum::routing::get(handle_studio_traces_with_store),
        )
        .route(
            "/studio/traces",
            axum::routing::get(handle_studio_traces_with_store),
        )
        // API endpoints
        .route(
            "/api/radar",
            axum::routing::get(crate::radar_visualizer::api_radar_handler),
        )
        .route(
            "/metrics",
            axum::routing::get(rullst_core::radar::prometheus_metrics_handler),
        )
        .route(
            "/api/revenue",
            axum::routing::get(crate::revenue_dashboard::api_revenue_handler),
        )
        .route(
            "/api/traces",
            axum::routing::get(handle_studio_traces_with_store),
        )
        .with_state(trace_store)
}

pub trait IntoStudioPort {
    fn into_port(self) -> u16;
}

impl IntoStudioPort for u16 {
    fn into_port(self) -> u16 {
        self
    }
}

impl IntoStudioPort for &str {
    fn into_port(self) -> u16 {
        if self.trim().is_empty() {
            5555
        } else {
            self.trim().parse::<u16>().unwrap_or(5555)
        }
    }
}

impl IntoStudioPort for String {
    fn into_port(self) -> u16 {
        self.as_str().into_port()
    }
}

impl IntoStudioPort for Option<u16> {
    fn into_port(self) -> u16 {
        self.unwrap_or(5555)
    }
}

/// Run Rullst Studio standalone dev server on specified port (default 5555)
pub async fn run_studio(
    port: impl IntoStudioPort,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port_num = port.into_port();
    let app = crate::Studio::new().into_router(crate::LocalStudioAccess::loopback_only())?;
    let addr = format!("127.0.0.1:{}", port_num);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}
