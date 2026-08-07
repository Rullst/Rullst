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
    for &b in bytes {
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

    #[test]
    fn test_inspect_json_payload_valid() {
        let json = b"{\"user\": {\"name\": \"Alice\", \"roles\": [\"admin\", \"user\"]}}";
        assert!(inspect_json_payload(json, 10, 1024).is_ok());
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
}
