//! AI Playground handler.

use super::super::layout::*;
use axum::response::{Html, IntoResponse};

pub async fn handle_studio_tools_ai(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let content = crate::ai_playground::render_ai_playground_html();
    if is_htmx {
        Html(format!("{}{}", content, render_sidebar_oob(&[], None))).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}
