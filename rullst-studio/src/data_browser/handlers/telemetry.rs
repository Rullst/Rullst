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
    let content = crate::revenue_dashboard::render_revenue_dashboard_page();
    if is_htmx {
        Html(content).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
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
