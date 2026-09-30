use super::*;
use axum::{Router, http::Request, middleware, routing::post};
use tower::ServiceExt;

#[test]
fn test_inspect_json_payload_valid() {
    let json = b"{\"user\": {\"name\": \"Alice\", \"roles\": [\"admin\", \"user\"]}}";
    assert!(inspect_json_payload(json, 10, 1024).is_ok());
}

#[test]
fn test_inspect_json_payload_braces_in_strings() {
    // String with 50 braces should not trigger a depth limit of 5
    let json = br#"{"snippet": "{{{{{{{{{{[[[[[[[[[[}}}}}}}}}}]]]]]]]]]]", "status": 200}"#;
    assert!(inspect_json_payload(json, 5, 1024).is_ok());
}

#[test]
fn test_inspect_json_payload_excessive_depth() {
    let mut deep_json = Vec::new();
    for _ in 0..40 {
        deep_json.extend_from_slice(b"{\"a\":");
    }
    deep_json.extend_from_slice(b"1");
    for _ in 0..40 {
        deep_json.extend_from_slice(b"}");
    }

    let res = inspect_json_payload(&deep_json, 10, 1024 * 1024);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        "Payload exceeds maximum allowed object nesting depth"
    );
}

#[test]
fn test_inspect_json_payload_too_large() {
    let json = vec![b'a'; 2000];
    let res = inspect_json_payload(&json, 10, 1000);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Payload exceeds maximum allowed size");
}

#[test]
fn syntax_and_duplicate_keys_are_rejected_recursively() {
    assert_eq!(
        inspect_json_payload(br#"{"role":"user","role":"admin"}"#, 10, 1_024),
        Err("JSON object contains a duplicate key")
    );
    assert_eq!(
        inspect_json_payload(br#"{"outer":{"id":1,"id":2}}"#, 10, 1_024),
        Err("JSON object contains a duplicate key")
    );
    assert_eq!(
        inspect_json_payload(br#"{"unterminated":true"#, 10, 1_024),
        Err("Payload is not valid JSON")
    );
}

#[test]
fn content_type_matching_is_exact_and_supports_json_suffixes() {
    assert!(is_json_content_type("application/json; charset=utf-8"));
    assert!(is_json_content_type("application/problem+json"));
    assert!(!is_json_content_type("text/application/json"));
    assert!(!is_json_content_type("application/json-malicious"));
}

#[test]
fn content_type_matching_accepts_every_json_form_axum_parses() {
    for content_type in [
        "APPLICATION/JSON",
        "application/vnd.api+JSON",
        "Application/problem+json; charset=utf-8",
        "application/json+patch",
    ] {
        assert!(is_json_content_type(content_type), "{content_type}");
    }
    for content_type in ["application/xml", "text/json", "application/jsonp"] {
        assert!(!is_json_content_type(content_type), "{content_type}");
    }
}

#[tokio::test]
async fn case_and_suffix_variants_cannot_skip_duplicate_key_checks() {
    for content_type in ["application/vnd.api+JSON", "Application/problem+json"] {
        let response = guarded_app()
            .oneshot(
                Request::post("/")
                    .header(axum::http::header::CONTENT_TYPE, content_type)
                    .body(Body::from(r#"{"role":"user","role":"admin"}"#))
                    .expect("request should be valid"),
            )
            .await
            .expect("middleware request should complete");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{content_type}");
    }
}

fn guarded_app() -> Router {
    Router::new()
        .route("/", post(|body: String| async move { body }))
        .layer(middleware::from_fn(schema_guard_middleware))
}

#[tokio::test]
async fn middleware_preserves_valid_json_and_non_json_bodies() {
    for (content_type, body) in [
        ("application/json; charset=utf-8", r#"{"safe":[1,2,3]}"#),
        ("text/plain", "not JSON { and deliberately unbalanced"),
    ] {
        let response = guarded_app()
            .oneshot(
                Request::post("/")
                    .header(axum::http::header::CONTENT_TYPE, content_type)
                    .body(Body::from(body))
                    .expect("request should be valid"),
            )
            .await
            .expect("middleware request should complete");
        assert_eq!(response.status(), StatusCode::OK);
        let returned = axum::body::to_bytes(response.into_body(), 1_024)
            .await
            .expect("response body should be readable");
        assert_eq!(returned.as_ref(), body.as_bytes());
    }
}

#[tokio::test]
async fn middleware_rejects_deep_and_oversized_json() {
    let deep = format!("{}0{}", "[".repeat(33), "]".repeat(33));
    let response = guarded_app()
        .oneshot(
            Request::post("/")
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(deep))
                .expect("request should be valid"),
        )
        .await
        .expect("middleware request should complete");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = guarded_app()
        .oneshot(
            Request::post("/")
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(vec![b' '; DEFAULT_MAX_PAYLOAD_BYTES + 2]))
                .expect("request should be valid"),
        )
        .await
        .expect("middleware request should complete");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

fn schema_bound_app() -> Router {
    let policy = JsonSchemaPolicy::from_schema(serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {"name": {"type": "string", "minLength": 1}},
        "required": ["name"],
        "additionalProperties": false
    }))
    .expect("route schema");
    Router::new().route(
        "/",
        post(|body: String| async move { body }).layer(middleware::from_fn_with_state(
            policy,
            json_schema_guard_middleware,
        )),
    )
}

#[tokio::test]
async fn compiled_schema_middleware_preserves_valid_bodies_and_fails_closed() {
    let valid = schema_bound_app()
        .oneshot(
            Request::post("/")
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"Ada"}"#))
                .expect("valid request"),
        )
        .await
        .expect("valid schema response");
    assert_eq!(valid.status(), StatusCode::OK);
    let returned = axum::body::to_bytes(valid.into_body(), 1_024)
        .await
        .expect("preserved body");
    assert_eq!(returned, r#"{"name":"Ada"}"#);

    for (content_type, body, expected) in [
        (
            "application/json",
            r#"{"name":"Ada","role":"admin"}"#,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "application/json",
            r#"{"name":"Ada","name":"Mallory"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "text/plain",
            r#"{"name":"Ada"}"#,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
    ] {
        let response = schema_bound_app()
            .oneshot(
                Request::post("/")
                    .header(axum::http::header::CONTENT_TYPE, content_type)
                    .body(Body::from(body))
                    .expect("schema rejection request"),
            )
            .await
            .expect("schema rejection response");
        assert_eq!(response.status(), expected);
    }
}
