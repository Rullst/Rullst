use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// RASP (Runtime Application Self-Protection) Inspector.
/// Analyzes request telemetry in zero-latency to block SQLi, Path Traversal, SSRF, and RCE.
pub struct RaspInspector;

impl RaspInspector {
    /// Inspects an incoming request target URI and query string for attack patterns.
    pub fn inspect_uri(uri: &str) -> bool {
        let lower = uri.to_lowercase();

        // SQL Injection patterns
        if lower.contains("union select")
            || lower.contains("' or '1'='1")
            || lower.contains("; drop table")
        {
            return true;
        }

        // Path Traversal patterns
        if lower.contains("../") || lower.contains("..\\") || lower.contains("/etc/passwd") {
            return true;
        }

        // SSRF patterns
        if lower.contains("169.254.169.254") || lower.contains("metadata.google.internal") {
            return true;
        }

        // RCE patterns
        if lower.contains("; cat ") || lower.contains("| sh") || lower.contains("; rm -rf") {
            return true;
        }

        false
    }
}

/// Tower Layer for RASP Runtime Application Self-Protection middleware.
#[derive(Clone, Default)]
pub struct RaspSecurityLayer;

impl<S> Layer<S> for RaspSecurityLayer {
    type Service = RaspSecurityService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RaspSecurityService { inner }
    }
}

/// Tower Service for RASP middleware.
#[derive(Clone)]
pub struct RaspSecurityService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for RaspSecurityService<S>
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
        let uri_str = req.uri().to_string();

        if RaspInspector::inspect_uri(&uri_str) {
            let store = crate::telemetry::SecurityStore::global();
            if let Ok(mut events) = store.live_events.lock() {
                events.insert(
                    0,
                    crate::telemetry::LiveSecurityEvent {
                        event_type: "RASP_PAYLOAD_INTERCEPTED".to_string(),
                        details: format!("Intercepted malicious payload in URI: {}", uri_str),
                        client_ip: "127.0.0.1".to_string(),
                        timestamp_str: crate::telemetry::current_timestamp_str(),
                        verified_hmac: true,
                    },
                );
            }
            let res = (
                StatusCode::FORBIDDEN,
                "🛡️ RASP Security Violation: Malicious payload intercepted.",
            )
                .into_response();
            return Box::pin(async move { Ok(res) });
        }

        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(req).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rasp_sqli_detection() {
        assert!(RaspInspector::inspect_uri(
            "/api/users?q=UNION SELECT * FROM passwords"
        ));
        assert!(RaspInspector::inspect_uri("/login?user=admin' OR '1'='1"));
        assert!(!RaspInspector::inspect_uri("/api/users?id=123"));
    }

    #[test]
    fn test_rasp_path_traversal_detection() {
        assert!(RaspInspector::inspect_uri(
            "/download?file=../../etc/passwd"
        ));
        assert!(!RaspInspector::inspect_uri("/download?file=document.pdf"));
    }
}
