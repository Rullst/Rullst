//! Telemetry handlers — Radar, Capital/Revenue, Distributed Traces.

use super::super::layout::*;
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};

pub async fn handle_studio_radar(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let content = crate::radar_visualizer::render_radar_page().await;
    if is_htmx {
        Html(content).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}

pub async fn handle_studio_capital(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    #[cfg(feature = "capital")]
    let content = crate::revenue_dashboard::render_revenue_dashboard_page();
    #[cfg(not(feature = "capital"))]
    let content = capital_not_enabled_page().to_string();
    if is_htmx {
        Html(content).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}

/// `/studio/capital` in a Studio built without its `capital` feature: the
/// application builds no `rullst-capital`, so there is nothing to show.
#[cfg(not(feature = "capital"))]
pub(crate) fn capital_not_enabled_page() -> &'static str {
    r#"<div class="p-8 font-mono space-y-4 max-w-3xl mx-auto">
            <h1 class="text-2xl font-extrabold text-white tracking-tight">Rullst Capital is not enabled</h1>
            <p class="text-slate-400 text-sm">The revenue dashboard reads <code>rullst-capital</code>, which this application does not build. Enable the <code>capital</code> feature of the <code>rullst</code> dependency (or of <code>rullst-studio</code>) to show it.</p>
        </div>"#
}

pub async fn handle_studio_traces(headers: axum::http::HeaderMap) -> impl IntoResponse {
    handle_studio_traces_with_store(
        State(crate::distributed_traces::DistributedTraceStore::default()),
        headers,
    )
    .await
}

pub async fn handle_studio_traces_with_store(
    State(store): State<crate::distributed_traces::DistributedTraceStore>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let Html(content) = crate::traces_visualizer::render_traces_page_with_store(&store).await;
    if is_htmx {
        Html(content).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}
