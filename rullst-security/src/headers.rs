//! Secure response headers with a CSP nonce shared with the request renderer.

use crate::telemetry::SecurityStore;
use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Request, Response, header},
};
pub use rullst_core::security::CspNonce;
use rullst_core::security::render_csp_policy;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Configuration options for the OWASP Secure Headers suite.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SecureHeadersConfig {
    /// Strict-Transport-Security (HSTS) header value.
    pub hsts: Option<String>,
    /// X-Frame-Options header value.
    pub frame_options: Option<String>,
    /// X-Content-Type-Options header value.
    pub content_type_options: Option<String>,
    /// Referrer-Policy header value.
    pub referrer_policy: Option<String>,
    /// Permissions-Policy header value.
    pub permissions_policy: Option<String>,
    /// Cross-Origin-Opener-Policy (COOP) header value.
    pub coop: Option<String>,
    /// Cross-Origin-Embedder-Policy (COEP) header value.
    pub coep: Option<String>,
    /// Cross-Origin-Resource-Policy (CORP) header value.
    pub corp: Option<String>,
    /// Whether to generate dynamic CSP with request nonce.
    pub dynamic_csp: bool,
    /// Optional CSP template. `{NONCE}` is replaced with the per-request nonce when enabled.
    /// `None` selects Rullst's strict shared default policy.
    pub csp: Option<String>,
}

impl Default for SecureHeadersConfig {
    fn default() -> Self {
        Self {
            hsts: Some("max-age=63072000; includeSubDomains; preload".to_string()),
            frame_options: Some("DENY".to_string()),
            content_type_options: Some("nosniff".to_string()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: Some(
                "camera=(), microphone=(), geolocation=(), payment=(), usb=()".to_string(),
            ),
            coop: Some("same-origin".to_string()),
            coep: Some("require-corp".to_string()),
            corp: Some("same-origin".to_string()),
            dynamic_csp: true,
            csp: None,
        }
    }
}

impl SecureHeadersConfig {
    /// Replaces the default CSP with a custom policy template.
    pub fn with_csp(mut self, csp: impl Into<String>) -> Self {
        self.csp = Some(csp.into());
        self
    }

    /// Uses a static CSP without generating a request nonce.
    pub fn without_dynamic_csp(mut self) -> Self {
        self.dynamic_csp = false;
        self
    }
}

fn insert_configured_header(
    headers: &mut HeaderMap,
    name: HeaderName,
    configured: Option<&String>,
) {
    if let Some(configured) = configured
        && let Ok(value) = HeaderValue::from_str(configured)
    {
        headers.insert(name, value);
    }
}

/// Tower Layer that applies the OWASP Secure Headers suite to all outgoing responses.
#[derive(Clone, Debug, Default)]
pub struct SecureHeadersLayer {
    config: SecureHeadersConfig,
}

impl SecureHeadersLayer {
    /// Creates a new `SecureHeadersLayer` with strict production defaults.
    pub fn new() -> Self {
        Self {
            config: SecureHeadersConfig::default(),
        }
    }

    /// Creates a custom SecureHeadersLayer with given configuration.
    pub fn with_config(config: SecureHeadersConfig) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for SecureHeadersLayer {
    type Service = SecureHeadersService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecureHeadersService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Tower Service for applying OWASP secure headers.
#[derive(Clone)]
pub struct SecureHeadersService<S> {
    inner: S,
    config: SecureHeadersConfig,
}

impl<S> Service<Request<Body>> for SecureHeadersService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let config = self.config.clone();
        let nonce = if config.dynamic_csp {
            let nonce = CspNonce::generate();
            req.extensions_mut().insert(nonce.clone());
            Some(nonce)
        } else {
            None
        };

        let fut = self.inner.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            let headers = res.headers_mut();

            insert_configured_header(
                headers,
                header::STRICT_TRANSPORT_SECURITY,
                config.hsts.as_ref(),
            );
            insert_configured_header(
                headers,
                header::X_FRAME_OPTIONS,
                config.frame_options.as_ref(),
            );
            insert_configured_header(
                headers,
                header::X_CONTENT_TYPE_OPTIONS,
                config.content_type_options.as_ref(),
            );
            insert_configured_header(
                headers,
                header::REFERRER_POLICY,
                config.referrer_policy.as_ref(),
            );
            insert_configured_header(
                headers,
                HeaderName::from_static("permissions-policy"),
                config.permissions_policy.as_ref(),
            );
            insert_configured_header(
                headers,
                HeaderName::from_static("cross-origin-opener-policy"),
                config.coop.as_ref(),
            );
            insert_configured_header(
                headers,
                HeaderName::from_static("cross-origin-embedder-policy"),
                config.coep.as_ref(),
            );
            insert_configured_header(
                headers,
                HeaderName::from_static("cross-origin-resource-policy"),
                config.corp.as_ref(),
            );
            headers.insert(
                HeaderName::from_static("x-xss-protection"),
                HeaderValue::from_static("0"),
            );

            let csp = render_csp_policy(config.csp.as_deref(), nonce.as_ref());
            let csp_value = HeaderValue::from_str(&csp).unwrap_or_else(|_| {
                HeaderValue::from_str(&render_csp_policy(None, nonce.as_ref()))
                    .unwrap_or_else(|_| HeaderValue::from_static("default-src 'none'"))
            });
            headers.insert(header::CONTENT_SECURITY_POLICY, csp_value);

            SecurityStore::global().inc_secure_headers();

            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_secure_headers_config() {
        let cfg = SecureHeadersConfig::default();
        assert!(cfg.hsts.unwrap().contains("max-age="));
        assert_eq!(cfg.frame_options.unwrap(), "DENY");
        assert_eq!(cfg.content_type_options.unwrap(), "nosniff");
        assert_eq!(cfg.coop.unwrap(), "same-origin");
        assert_eq!(cfg.coep.unwrap(), "require-corp");
        assert_eq!(cfg.corp.unwrap(), "same-origin");
        assert!(cfg.dynamic_csp);
        assert!(cfg.csp.is_none());
    }

    #[tokio::test]
    async fn nonce_is_available_to_renderer_and_matches_header() {
        use axum::{Router, extract::Extension, routing::get};
        use tower::ServiceExt;

        let app = Router::new()
            .route(
                "/",
                get(|Extension(nonce): Extension<CspNonce>| async move { nonce.to_string() }),
            )
            .layer(SecureHeadersLayer::default());
        let response = app
            .oneshot(Request::new(Body::empty()))
            .await
            .expect("request should complete");
        let csp = response
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("CSP should be present")
            .to_str()
            .expect("CSP should be ASCII")
            .to_owned();
        assert!(!csp.contains("unsafe-inline"));
        assert!(!csp.contains("unsafe-eval"));
        let body = axum::body::to_bytes(response.into_body(), 1_024)
            .await
            .expect("body should be readable");
        let nonce = std::str::from_utf8(&body).expect("nonce should be UTF-8");
        assert!(csp.contains(&format!("'nonce-{nonce}'")));
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_secure_headers_config_initialization() {
        let config = SecureHeadersConfig::default();
        assert!(config.dynamic_csp);
        assert!(config.hsts.is_some());
    }
}
