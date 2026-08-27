use crate::telemetry::SecurityStore;
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Default maximum JSON payload size: 2MB
pub const DEFAULT_MAX_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
/// Default maximum object nesting depth: 32 levels
pub const DEFAULT_MAX_NESTING_DEPTH: usize = 32;

/// Inspects raw JSON bytes to verify maximum size and nesting depth to prevent JSON bomb DoS.
/// Correctly ignores braces inside JSON quoted strings to prevent false positives on string values.
pub fn inspect_json_payload(
    bytes: &[u8],
    max_depth: usize,
    max_bytes: usize,
) -> Result<(), &'static str> {
    if bytes.len() > max_bytes {
        SecurityStore::global().inc_schema_violations();
        return Err("Payload exceeds maximum allowed size");
    }

    let mut current_depth: usize = 0;
    let mut in_string = false;
    let mut escape = false;

    for &b in bytes {
        if escape {
            escape = false;
            continue;
        }

        if b == b'\\' && in_string {
            escape = true;
            continue;
        }

        if b == b'"' {
            in_string = !in_string;
            continue;
        }

        if !in_string {
            if b == b'{' || b == b'[' {
                current_depth += 1;
                if current_depth > max_depth {
                    SecurityStore::global().inc_schema_violations();
                    return Err("Payload exceeds maximum allowed object nesting depth");
                }
            } else if b == b'}' || b == b']' {
                current_depth = current_depth.saturating_sub(1);
            }
        }
    }

    Ok(())
}

/// Middleware that inspects application/json request payloads for JSON bombs and depth limits.
pub async fn schema_guard_middleware(req: Request, next: Next) -> Response {
    let content_type = req
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v: &HeaderValue| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("application/json") {
        let (parts, body) = req.into_parts();

        // Read request body up to MAX_PAYLOAD_BYTES + 1
        let bytes = match axum::body::to_bytes(body, DEFAULT_MAX_PAYLOAD_BYTES + 1).await {
            Ok(b) => b,
            Err(_) => {
                SecurityStore::global().inc_schema_violations();
                return (
                    StatusCode::BAD_REQUEST,
                    "Failed to read JSON request body or payload too large",
                )
                    .into_response();
            }
        };

        if let Err(err_msg) =
            inspect_json_payload(&bytes, DEFAULT_MAX_NESTING_DEPTH, DEFAULT_MAX_PAYLOAD_BYTES)
        {
            return (StatusCode::BAD_REQUEST, err_msg).into_response();
        }

        let reconstructed_req = Request::from_parts(parts, Body::from(bytes));
        next.run(reconstructed_req).await
    } else {
        next.run(req).await
    }
}

#[cfg(test)]
mod tests {
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
}
