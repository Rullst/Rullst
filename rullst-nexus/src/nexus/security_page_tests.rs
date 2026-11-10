#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::http::{HeaderMap, HeaderValue};
use rullst_security::SecurityStore;

fn state() -> Arc<NexusState> {
    Arc::new(NexusState {
        registry: Arc::new(Vec::new()),
        brand: Arc::new("Rullst <Admin>".to_string()),
        audit_policy: crate::nexus::NexusAuditPolicy::Disabled,
    })
}

#[test]
fn security_page_renders_real_bans_routes_event_classes_and_integrity_without_xss() {
    let store = SecurityStore::new();
    store.record_honeypot_trap("203.0.113.77", "/<script>alert(1)</script>");
    store.record_sanitization("neutralized <img src=x>");
    store.record_prompt_injection_blocked("203.0.113.78", "ignore <system>");
    store.record_rbac_denial("<admin>", "course<script>");

    let mut headers = HeaderMap::new();
    headers.insert("hx-request", HeaderValue::from_static("true"));
    let partial = render_security_page(&state(), &headers, &store).0;
    assert!(!partial.contains("<!DOCTYPE html>"));
    assert!(partial.contains("203.0.113.77"));
    // Only signed SIEM ingestion verifies events; the store keeps local ones.
    assert!(!partial.contains("HMAC VERIFIED"));
    assert!(partial.contains("UNSIGNED LOCAL EVENT"));
    assert!(partial.contains("HONEYPOT_TRAP_TRIGGERED"));
    assert!(partial.contains("XSS_PAYLOAD_NEUTRALIZED"));
    assert!(partial.contains("AI_PROMPT_INJECTION_SHIELDED"));
    assert!(partial.contains("RBAC_ACCESS_DENIED"));
    assert!(partial.contains("&lt;script&gt;"));
    assert!(!partial.contains("<script>alert(1)</script>"));

    let full = render_security_page(&state(), &HeaderMap::new(), &store).0;
    assert!(full.contains("<!DOCTYPE html>"));
    assert!(full.contains("Rullst &lt;Admin&gt;"));
    assert!(full.contains("nexus-nav-sec nexus-nav-active"));
}

#[tokio::test]
async fn the_handler_renders_the_process_wide_store() {
    let mut headers = HeaderMap::new();
    headers.insert("hx-request", HeaderValue::from_static("true"));
    let html = nexus_security_page(State(state()), headers).await.0;
    assert!(html.contains("Threat Radar"));
}

#[test]
fn empty_security_page_is_explicit_about_unmounted_and_unavailable_sources() {
    let mut headers = HeaderMap::new();
    headers.insert("hx-request", HeaderValue::from_static("true"));
    let html = render_security_page(&state(), &headers, &SecurityStore::new()).0;
    assert!(html.contains("No IP addresses currently banned by WAF"));
    assert!(html.contains("Available default; mount middleware to arm"));
    assert!(html.contains("No in-process security events recorded"));
    assert!(html.contains(AUDIT_CHAIN_UNAVAILABLE));
}

#[test]
fn event_types_map_to_their_tone_classes() {
    assert_eq!(event_tone("HONEYPOT_TRAP_TRIGGERED"), "nexus-tone-rose");
    assert_eq!(event_tone("XSS_PAYLOAD_NEUTRALIZED"), "nexus-tone-cyan");
    assert_eq!(
        event_tone("AI_PROMPT_INJECTION_SHIELDED"),
        "nexus-tone-violet"
    );
    assert_eq!(event_tone("RBAC_ACCESS_DENIED"), "nexus-tone-amber");
    assert_eq!(
        event_integrity_badge(true),
        ("HMAC VERIFIED", "nexus-badge-verified")
    );
    assert_eq!(
        event_integrity_badge(false),
        ("UNSIGNED LOCAL EVENT", "nexus-badge-unsigned")
    );
}
