//! The assistant's routes in a build without the `ai` feature: a notice that
//! names the feature, which the Nexus router answers with 404.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Html;
use std::sync::Arc;

use crate::nexus::types::NexusState;
use crate::nexus::ui::{render_shell, render_sidebar, wants_fragment};

const NOTICE: &str = "<p>The AI assistant is not enabled in this build of Nexus.</p>\
     <p>Enable the <code class=\"nexus-code\">nexus-ai</code> feature of \
     <code class=\"nexus-code\">rullst</code> (or the <code class=\"nexus-code\">ai</code> \
     feature of <code class=\"nexus-code\">rullst-nexus</code>) and rebuild the application; \
     the assistant then answers through the AI providers configured in its environment.</p>";

/// The notice page, as a fragment for htmx requests.
pub(crate) fn page_html(state: &NexusState, headers: &HeaderMap) -> String {
    let content = format!(
        "<div class=\"nexus-page-header\">\
         <h1 class=\"nexus-page-title\">&#129302; AI assistant not enabled</h1></div>\
         <div class=\"nexus-card\">{NOTICE}</div>"
    );
    if wants_fragment(headers) {
        content
    } else {
        render_shell(state, &render_sidebar(state, None), &content)
    }
}

/// The notice as an assistant chat bubble.
pub(crate) fn query_html() -> String {
    format!(
        "<div class=\"nexus-chat-bubble nexus-chat-assistant\">\
         <span class=\"nexus-chat-avatar\">&#129302;</span>\
         <div class=\"nexus-chat-text\">{NOTICE}</div></div>"
    )
}

/// `GET /chat` of the Nexus router.
pub(crate) async fn page(
    State(state): State<Arc<NexusState>>,
    headers: HeaderMap,
) -> (StatusCode, Html<String>) {
    (StatusCode::NOT_FOUND, Html(page_html(&state, &headers)))
}

/// `POST /chat/query` of the Nexus router.
pub(crate) async fn query() -> (StatusCode, Html<String>) {
    (StatusCode::NOT_FOUND, Html(query_html()))
}
