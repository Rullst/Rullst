//! OWASP Secure Headers Suite & Dynamic Hardening Layer.
//! Provides out-of-the-box A+ rating on security audits with HSTS, CSP nonces, Permissions-Policy, COOP, COEP, and CORP.

use crate::sanitizer::csp::generate_nonce;
use crate::telemetry::SecurityStore;
use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request, Response},
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Configuration options for the OWASP Secure Headers suite.
#[derive(Clone, Debug)]
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
        }
    }
}

/// Tower Layer that applies the OWASP Secure Headers suite to all outgoing responses.
#[derive(Clone, Debug, Default)]
pub struct SecureHeadersLayer {
    config: SecureHeadersConfig,
}

impl SecureHeadersLayer {
    /// Creates a new SecureHeadersLayer with default production A+ settings.
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

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let config = self.config.clone();
        let nonce = if config.dynamic_csp {
            Some(generate_nonce())
        } else {
            None
        };

        let fut = self.inner.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            let headers = res.headers_mut();

            if let Some(ref hsts_val) = config.hsts
                && let Ok(v) = HeaderValue::from_str(hsts_val)
            {
                headers.insert(HeaderName::from_static("strict-transport-security"), v);
            }

            if let Some(ref frame_opt) = config.frame_options
                && let Ok(v) = HeaderValue::from_str(frame_opt)
            {
                headers.insert(HeaderName::from_static("x-frame-options"), v);
            }

            if let Some(ref content_type_opt) = config.content_type_options
                && let Ok(v) = HeaderValue::from_str(content_type_opt)
            {
                headers.insert(HeaderName::from_static("x-content-type-options"), v);
            }

            if let Some(ref referrer_pol) = config.referrer_policy
                && let Ok(v) = HeaderValue::from_str(referrer_pol)
            {
                headers.insert(HeaderName::from_static("referrer-policy"), v);
            }

            if let Some(ref permissions_pol) = config.permissions_policy
                && let Ok(v) = HeaderValue::from_str(permissions_pol)
            {
                headers.insert(HeaderName::from_static("permissions-policy"), v);
            }

            if let Some(ref coop_opt) = config.coop
                && let Ok(v) = HeaderValue::from_str(coop_opt)
            {
                headers.insert(HeaderName::from_static("cross-origin-opener-policy"), v);
            }

            if let Some(ref coep_opt) = config.coep
                && let Ok(v) = HeaderValue::from_str(coep_opt)
            {
                headers.insert(HeaderName::from_static("cross-origin-embedder-policy"), v);
            }

            if let Some(ref corp_opt) = config.corp
                && let Ok(v) = HeaderValue::from_str(corp_opt)
            {
                headers.insert(HeaderName::from_static("cross-origin-resource-policy"), v);
            }

            if let Some(n) = nonce {
                let csp_val = format!(
                    "default-src 'self'; script-src 'self' 'nonce-{}'; style-src 'self' 'unsafe-inline'; object-src 'none'; base-uri 'self';",
                    n
                );
                if let Ok(v) = HeaderValue::from_str(&csp_val) {
                    headers.insert(HeaderName::from_static("content-security-policy"), v);
                }
            }

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
