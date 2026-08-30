use crate::telemetry::SecurityStore;
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use std::{collections::HashSet, fmt};

/// Default maximum JSON payload size: 2MB
pub const DEFAULT_MAX_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
/// Default maximum object nesting depth: 32 levels
pub const DEFAULT_MAX_NESTING_DEPTH: usize = 32;
const DUPLICATE_KEY_MARKER: &str = "duplicate JSON object key";

struct StrictJson;

impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictJsonVisitor)
    }
}

struct StrictJsonVisitor;

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = StrictJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key) {
                return Err(de::Error::custom(DUPLICATE_KEY_MARKER));
            }
            map.next_value::<StrictJson>()?;
        }
        Ok(StrictJson)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<StrictJson>()?.is_some() {}
        Ok(StrictJson)
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(StrictJson)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(StrictJson)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(StrictJson)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(StrictJson)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(StrictJson)
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(StrictJson)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictJson)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictJson)
    }
}

/// Validates JSON syntax, duplicate object keys, size and nesting depth.
///
/// This is a transport guard, not an OpenAPI/JSON Schema validator.
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

    if let Err(error) = serde_json::from_slice::<StrictJson>(bytes) {
        SecurityStore::global().inc_schema_violations();
        if error.to_string().contains(DUPLICATE_KEY_MARKER) {
            return Err("JSON object contains a duplicate key");
        }
        return Err("Payload is not valid JSON");
    }

    Ok(())
}

fn is_json_content_type(value: &str) -> bool {
    let media_type = value.split(';').next().map(str::trim).unwrap_or_default();
    media_type.eq_ignore_ascii_case("application/json")
        || media_type
            .strip_suffix("+json")
            .is_some_and(|prefix| prefix.starts_with("application/"))
}

/// Middleware that inspects application/json request payloads for JSON bombs and depth limits.
pub async fn schema_guard_middleware(req: Request, next: Next) -> Response {
    let content_type = req
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v: &HeaderValue| v.to_str().ok())
        .unwrap_or("");

    if is_json_content_type(content_type) {
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
