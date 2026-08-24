//! Data Loss Prevention (DLP) Response Interceptor.
//! Intercepts HTTP response streams to prevent accidental leaks of private keys, AWS credentials, and database secrets.

use crate::telemetry::{LiveSecurityEvent, SecurityStore, current_timestamp_str};
use axum::{
    body::Body,
    http::{Request, Response},
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Masks sensitive patterns from response payloads. Returns (sanitized_bytes, was_masked).
pub fn mask_response_payload(input: &[u8]) -> (Vec<u8>, bool) {
    if input.is_empty() {
        return (Vec::new(), false);
    }

    let text = String::from_utf8_lossy(input);
    let mut sanitized = text.to_string();
    let mut modified = false;

    // 1. Mask Private Keys (RSA / OpenSSH / Generic)
    if sanitized.contains("-----BEGIN PRIVATE KEY-----")
        || sanitized.contains("-----BEGIN RSA PRIVATE KEY-----")
        || sanitized.contains("-----BEGIN OPENSSH PRIVATE KEY-----")
    {
        for prefix in &[
            "-----BEGIN PRIVATE KEY-----",
            "-----BEGIN RSA PRIVATE KEY-----",
            "-----BEGIN OPENSSH PRIVATE KEY-----",
        ] {
            while let Some(start) = sanitized.find(prefix) {
                let end = sanitized[start..]
                    .find("-----END")
                    .map(|e| {
                        let sub = &sanitized[start + e..];
                        sub.find("-----")
                            .map(|e2| start + e + e2 + 5)
                            .unwrap_or(sanitized.len())
                    })
                    .unwrap_or(sanitized.len());

                sanitized.replace_range(start..end, "[DLP_BLOCKED_PRIVATE_KEY]");
                modified = true;
            }
        }
    }

    // 2. Mask all AWS Access Keys (AKIA...)
    let mut cursor = 0;
    while let Some(offset) = sanitized[cursor..].find("AKIA") {
        let start = cursor + offset;
        let end = (start + 20).min(sanitized.len());
        if !sanitized[start..end].starts_with("AKIA****************") {
            sanitized.replace_range(start..end, "AKIA****************");
            modified = true;
        }
        cursor = start + 20.min(sanitized.len().saturating_sub(start));
        if cursor >= sanitized.len() {
            break;
        }
    }

    // 3. Mask all database connection string passwords (postgres://user:pass@host:5432/db)
    for scheme in &["postgres://", "postgresql://", "mysql://", "redis://"] {
        let mut cursor = 0;
        while let Some(offset) = sanitized[cursor..].find(scheme) {
            let start = cursor + offset;
            let rest = &sanitized[start + scheme.len()..];
            if let Some(at_idx) = rest.find('@') {
                let auth_part = &rest[..at_idx];
                if let Some(colon_idx) = auth_part.find(':') {
                    let pass_start = start + scheme.len() + colon_idx + 1;
                    let pass_end = start + scheme.len() + at_idx;
                    if &sanitized[pass_start..pass_end] != "*****" {
                        sanitized.replace_range(pass_start..pass_end, "*****");
                        modified = true;
                    }
                    cursor = start + scheme.len() + at_idx + 1;
                    if cursor >= sanitized.len() {
                        break;
                    }
                    continue;
                }
            }
            cursor = start + scheme.len();
            if cursor >= sanitized.len() {
                break;
            }
        }
    }

    if modified {
        let store = SecurityStore::global();
        store.inc_dlp_masked();

        if let Ok(mut events) = store.live_events.lock() {
            events.insert(
                0,
                LiveSecurityEvent {
                    event_type: "DLP_SECRET_LEAK_PREVENTED".to_string(),
                    details: "Neutralized secret credentials/key from outgoing HTTP response"
                        .to_string(),
                    client_ip: "127.0.0.1".to_string(),
                    timestamp_str: current_timestamp_str(),
                    verified_hmac: true,
                },
            );
            if events.len() > 50 {
                events.truncate(50);
            }
        }
        (sanitized.into_bytes(), true)
    } else {
        (input.to_vec(), false)
    }
}

/// Tower Layer for Data Loss Prevention (DLP) response interception.
#[derive(Clone, Default)]
pub struct DlpResponseLayer;

impl<S> Layer<S> for DlpResponseLayer {
    type Service = DlpResponseService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DlpResponseService { inner }
    }
}

/// Tower Service for DLP response interception.
#[derive(Clone)]
pub struct DlpResponseService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for DlpResponseService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let res = inner.call(req).await?;
            let (parts, body) = res.into_parts();

            // Collect response body up to 2MB limit
            let bytes = match axum::body::to_bytes(body, 2 * 1024 * 1024).await {
                Ok(b) => b,
                Err(_) => return Ok(Response::from_parts(parts, Body::empty())),
            };

            let (sanitized_bytes, _) = mask_response_payload(&bytes);
            let new_body = Body::from(sanitized_bytes);

            Ok(Response::from_parts(parts, new_body))
        })
    }
}

/// Alias for DlpResponseLayer
pub type DlpLayer = DlpResponseLayer;

/// Alias for DlpResponseService
pub type DlpService<S> = DlpResponseService<S>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_private_key() {
        let payload = b"Error: key was -----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA...\n-----END RSA PRIVATE KEY----- in config";
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(was_modified);
        let masked_str = String::from_utf8(masked).unwrap();
        assert!(masked_str.contains("[DLP_BLOCKED_PRIVATE_KEY]"));
        assert!(!masked_str.contains("BEGIN RSA PRIVATE KEY"));
    }

    #[test]
    fn test_mask_database_url() {
        let payload =
            b"{\"db\": \"postgres://admin:super_secret_password_123@localhost:5432/app\"}";
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(was_modified);
        let masked_str = String::from_utf8(masked).unwrap();
        assert!(masked_str.contains("postgres://admin:*****@localhost:5432/app"));
        assert!(!masked_str.contains("super_secret_password_123"));
    }

    #[test]
    fn test_clean_payload_untouched() {
        let payload = b"{\"message\": \"Hello Rullst!\", \"status\": 200}";
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(!was_modified);
        assert_eq!(masked, payload);
    }

    #[test]
    fn test_mask_aws_access_key() {
        let payload = b"{\"aws_key\": \"AKIAIOSFODNN7EXAMPLE\"}";
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(was_modified);
        let masked_str = String::from_utf8(masked).unwrap();
        assert!(masked_str.contains("AKIA****************"));
        assert!(!masked_str.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_mask_mysql_and_redis_urls() {
        let payload = b"mysql://root:my_secret_sql_pass@127.0.0.1:3306/db and redis://default:redis_auth_token@cache:6379";
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(was_modified);
        let masked_str = String::from_utf8(masked).unwrap();
        assert!(masked_str.contains("mysql://root:*****@127.0.0.1:3306/db"));
        assert!(masked_str.contains("redis://default:*****@cache:6379"));
    }

    #[test]
    fn test_mask_multiple_keys_and_dsns() {
        let payload = br#"{"keys": ["AKIA1111111111111111", "AKIA2222222222222222"], "dbs": ["postgres://u1:p1@h1:5432/d1", "postgres://u2:p2@h2:5432/d2"]}"#;
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(was_modified);
        let masked_str = String::from_utf8(masked).unwrap();
        assert!(!masked_str.contains("AKIA1111111111111111"));
        assert!(!masked_str.contains("AKIA2222222222222222"));
        assert!(!masked_str.contains(":p1@"));
        assert!(!masked_str.contains(":p2@"));
        assert!(masked_str.contains("postgres://u1:*****@h1:5432/d1"));
        assert!(masked_str.contains("postgres://u2:*****@h2:5432/d2"));
    }

    #[tokio::test]
    async fn test_dlp_layer_middleware() {
        use axum::http::{Request, StatusCode};
        use axum::response::IntoResponse;
        use axum::routing::get;
        use tower::ServiceExt;

        async fn handler() -> impl IntoResponse {
            (
                StatusCode::OK,
                "Config leaked: postgres://user:secret123@db:5432/main",
            )
        }

        let app = axum::Router::new()
            .route("/secret", get(handler))
            .layer(DlpResponseLayer);

        let req = Request::builder()
            .uri("/secret")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 10000).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("postgres://user:*****@db:5432/main"));
        assert!(!body_str.contains("secret123"));
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_dlp_empty_payload() {
        let (masked, modified) = mask_response_payload(&[]);
        assert!(!modified);
        assert!(masked.is_empty());
    }
}
