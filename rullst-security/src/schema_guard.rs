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

mod policy;
pub use policy::{JsonSchemaPolicy, SchemaPolicyError};

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

/// Accepts every JSON media type axum's `Json` extractor accepts, in any
/// ASCII case, so a body the handler parses cannot skip these checks.
fn is_json_content_type(value: &str) -> bool {
    crate::media_type::is_json(crate::media_type::essence(value))
}

/// Methods whose requests normally carry no body. Clients and fetch wrappers
/// often send a JSON `Content-Type` on them anyway, so an empty body is passed
/// through instead of being rejected as invalid JSON.
fn is_bodiless_method(method: &axum::http::Method, include_delete: bool) -> bool {
    matches!(
        *method,
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    ) || (include_delete && *method == axum::http::Method::DELETE)
}

/// Middleware that inspects application/json request payloads for JSON bombs and depth limits.
///
/// An empty `GET`, `HEAD`, `DELETE` or `OPTIONS` body passes through even with
/// a JSON `Content-Type`; a handler that requires a body still rejects it.
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

        if !(bytes.is_empty() && is_bodiless_method(&parts.method, true))
            && let Err(err_msg) =
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

/// Route-scoped middleware that applies the strict transport checks and then a
/// precompiled JSON Schema 2020-12 policy supplied by the application.
///
/// Mount this with `axum::middleware::from_fn_with_state`. The policy disables
/// filesystem and network reference resolution and accepts only local `$ref`
/// and `$dynamicRef` values. `GET`, `HEAD` and `OPTIONS` requests pass through
/// without a JSON `Content-Type` or with an empty body.
pub async fn json_schema_guard_middleware(
    axum::extract::State(policy): axum::extract::State<JsonSchemaPolicy>,
    req: Request,
    next: Next,
) -> Response {
    let content_type = req
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !is_json_content_type(content_type) {
        if is_bodiless_method(req.method(), false) {
            return next.run(req).await;
        }
        SecurityStore::global().inc_schema_violations();
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "This route requires a JSON Content-Type",
        )
            .into_response();
    }

    let (parts, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, DEFAULT_MAX_PAYLOAD_BYTES + 1).await {
        Ok(bytes) => bytes,
        Err(_) => {
            SecurityStore::global().inc_schema_violations();
            return (
                StatusCode::BAD_REQUEST,
                "Failed to read JSON request body or payload too large",
            )
                .into_response();
        }
    };
    if bytes.is_empty() && is_bodiless_method(&parts.method, false) {
        return next
            .run(Request::from_parts(parts, Body::from(bytes)))
            .await;
    }
    if let Err(message) =
        inspect_json_payload(&bytes, DEFAULT_MAX_NESTING_DEPTH, DEFAULT_MAX_PAYLOAD_BYTES)
    {
        return (StatusCode::BAD_REQUEST, message).into_response();
    }
    let instance = match serde_json::from_slice(&bytes) {
        Ok(instance) => instance,
        Err(_) => {
            SecurityStore::global().inc_schema_violations();
            return (StatusCode::BAD_REQUEST, "Payload is not valid JSON").into_response();
        }
    };
    if policy.validate(&instance).is_err() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "JSON payload does not match the configured schema",
        )
            .into_response();
    }

    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

#[cfg(test)]
mod tests;
