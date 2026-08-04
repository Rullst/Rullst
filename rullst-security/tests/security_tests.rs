use rullst_security::{
    audit::{AuditChain, StdoutAuditLogger},
    honey::HoneypotState,
    rbac::{RbacGuard, UserContext},
    sanitizer::{HtmlSanitizer, csp::generate_nonce},
};
use std::sync::Arc;

#[tokio::test]
async fn test_honeypot_trap_detection() {
    let state = HoneypotState::default();

    assert!(state.is_trap("/.env"));
    assert!(state.is_trap("/admin.php"));
    assert!(!state.is_trap("/api/users"));

    assert!(!state.is_banned("192.168.1.100"));
    state.ban_ip("192.168.1.100".to_string());
    assert!(state.is_banned("192.168.1.100"));
    assert_eq!(state.banned_count(), 1);
}

#[test]
fn test_html_sanitizer_xss_prevention() {
    let dirty = "<script>alert('XSS')</script><b>Safe Text</b>";
    let clean = HtmlSanitizer::sanitize(dirty);
    assert!(!clean.contains("<script>"));
    assert!(clean.contains("<b>Safe Text</b>"));

    let escaped = HtmlSanitizer::sanitize_text("Hello <World> & \"Friends\"");
    assert_eq!(escaped, "Hello &lt;World&gt; &amp; &quot;Friends&quot;");
}

#[test]
fn test_csp_nonce_generation() {
    let nonce1 = generate_nonce();
    let nonce2 = generate_nonce();

    assert_eq!(nonce1.len(), 24); // 16 bytes base64 encoded
    assert_ne!(nonce1, nonce2);
}

#[test]
fn test_rbac_guard_authorization() {
    let user = UserContext::new("user_123", vec!["editor".to_string()]);
    let admin = UserContext::new("admin_999", vec!["admin".to_string()]);

    assert!(RbacGuard::authorize(&editor_or_admin(&user), "editor").is_ok());
    assert!(RbacGuard::authorize(&user, "admin").is_err());
    assert!(RbacGuard::authorize(&admin, "editor").is_ok());

    assert!(RbacGuard::authorize_owner_or_role(&user, "user_123", "admin").is_ok());
    assert!(RbacGuard::authorize_owner_or_role(&user, "user_456", "admin").is_err());
}

fn editor_or_admin(ctx: &UserContext) -> UserContext {
    ctx.clone()
}

#[tokio::test]
async fn test_audit_chain_hmac_integrity() {
    let logger = Arc::new(StdoutAuditLogger::default());
    let secret = b"super-secret-hmac-key";
    let chain = AuditChain::new(secret, logger);

    let record1 = chain
        .record_event("admin", "UPDATE", "user_123", "{\"role\":\"manager\"}")
        .await
        .unwrap();

    assert_eq!(record1.sequence_id, 1);
    assert_eq!(record1.previous_hash, "GENESIS_HASH");
    assert!(AuditChain::verify_record(secret, &record1));

    let record2 = chain
        .record_event("admin", "DELETE", "post_456", "{}")
        .await
        .unwrap();

    assert_eq!(record2.sequence_id, 2);
    assert_eq!(record2.previous_hash, record1.hash);
    assert!(AuditChain::verify_record(secret, &record2));

    // Tampered record test
    let mut tampered = record2.clone();
    tampered.payload = "{\"tampered\": true}".to_string();
    assert!(!AuditChain::verify_record(secret, &tampered));
}
