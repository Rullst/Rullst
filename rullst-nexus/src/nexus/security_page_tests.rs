#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::http::{HeaderMap, HeaderValue};
use rullst_security::LiveSecurityEvent;
use std::sync::LazyLock;

static SECURITY_PAGE_TEST_LOCK: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

fn state() -> Arc<NexusState> {
    Arc::new(NexusState {
        registry: Arc::new(Vec::new()),
        brand: Arc::new("Rullst <Admin>".to_string()),
        audit_policy: crate::nexus::NexusAuditPolicy::Disabled,
    })
}

fn clear_global_store() {
    let store = rullst_security::SecurityStore::global();
    store.banned_ips.clear();
    store.honeypot_route_hits.clear();
    store.live_events.lock().unwrap().clear();
}

#[tokio::test]
async fn security_page_renders_real_bans_routes_event_classes_and_integrity_without_xss() {
    let _guard = SECURITY_PAGE_TEST_LOCK.lock().await;
    clear_global_store();
    let store = rullst_security::SecurityStore::global();
    store.record_honeypot_trap("203.0.113.77", "/<script>alert(1)</script>");
    store.record_sanitization("neutralized <img src=x>");
    store.record_prompt_injection_blocked("203.0.113.78", "ignore <system>");
    store.record_rbac_denial("<admin>", "course<script>");
    let mut verified = LiveSecurityEvent::local(
        "SECURITY_EVENT",
        "verified <external> event",
        "203.0.113.79",
    );
    verified.verified_hmac = true;
    store.live_events.lock().unwrap().insert(0, verified);

    let mut headers = HeaderMap::new();
    headers.insert("hx-request", HeaderValue::from_static("true"));
    let partial = nexus_security_page(State(state()), headers).await.0;
    assert!(!partial.contains("<!DOCTYPE html>"));
    assert!(partial.contains("203.0.113.77"));
    assert!(partial.contains("HMAC VERIFIED"));
    assert!(partial.contains("UNSIGNED LOCAL EVENT"));
    assert!(partial.contains("HONEYPOT_TRAP_TRIGGERED"));
    assert!(partial.contains("XSS_PAYLOAD_NEUTRALIZED"));
    assert!(partial.contains("AI_PROMPT_INJECTION_SHIELDED"));
    assert!(partial.contains("RBAC_ACCESS_DENIED"));
    assert!(partial.contains("&lt;script&gt;"));
    assert!(!partial.contains("<script>alert(1)</script>"));

    let full = nexus_security_page(State(state()), HeaderMap::new())
        .await
        .0;
    assert!(full.contains("<!DOCTYPE html>"));
    assert!(full.contains("Rullst &lt;Admin&gt;"));
    assert!(full.contains("nexus-nav-sec nexus-nav-active"));

    clear_global_store();
}

#[tokio::test]
async fn empty_security_page_is_explicit_about_unmounted_and_unavailable_sources() {
    let _guard = SECURITY_PAGE_TEST_LOCK.lock().await;
    clear_global_store();
    let mut headers = HeaderMap::new();
    headers.insert("hx-request", HeaderValue::from_static("true"));
    let html = nexus_security_page(State(state()), headers).await.0;
    assert!(html.contains("No IP addresses currently banned by WAF"));
    assert!(html.contains("Available default; mount middleware to arm"));
    assert!(html.contains("No in-process security events recorded"));
    assert!(html.contains(AUDIT_CHAIN_UNAVAILABLE));
}
