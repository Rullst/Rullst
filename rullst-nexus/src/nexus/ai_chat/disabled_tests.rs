//! The not-enabled notice, in every build.

use super::*;
use crate::nexus::NexusAuditPolicy;
use axum::http::HeaderValue;

fn state() -> Arc<NexusState> {
    Arc::new(NexusState {
        registry: Arc::new(Vec::new()),
        brand: Arc::new("Notice Brand".to_string()),
        audit_policy: NexusAuditPolicy::Disabled,
    })
}

fn fragment_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("hx-request", HeaderValue::from_static("true"));
    headers
}

#[test]
fn the_page_names_the_feature_inside_the_shell_or_as_a_fragment() {
    let page = page_html(&state(), &HeaderMap::new());
    assert!(page.contains("AI assistant not enabled"), "{page}");
    assert!(page.contains(NOTICE), "{page}");
    assert!(
        page.contains("Notice Brand"),
        "the full page is the panel shell"
    );
    assert!(page.len() > NOTICE.len() * 2, "{page}");

    let fragment = page_html(&state(), &fragment_headers());
    assert!(fragment.starts_with("<div class=\"nexus-page-header\">"));
    assert!(fragment.contains(NOTICE));
    assert!(!fragment.contains("Notice Brand"), "{fragment}");
}

#[test]
fn the_query_answer_is_an_assistant_bubble_with_the_notice() {
    let answer = query_html();
    assert!(answer.starts_with("<div class=\"nexus-chat-bubble nexus-chat-assistant\">"));
    assert!(answer.contains(NOTICE));
    assert!(answer.contains("nexus-ai") && answer.contains("rullst-nexus"));
}

#[tokio::test]
async fn both_routes_answer_404_with_the_notice() {
    let (status, Html(page)) = page(State(state()), HeaderMap::new()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(page, page_html(&state(), &HeaderMap::new()));

    let (status, Html(answer)) = query().await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(answer, query_html());
}
