//! Secure response headers and per-request Content Security Policy nonces.

use axum::{
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue, header},
    middleware::Next,
    response::Response,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use rand::Rng;

pub use crate::config::DEFAULT_CSP_TEMPLATE;

const DEFAULT_STATIC_CSP: &str = "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; font-src 'self'; worker-src 'self' blob:";

/// Response extension marking that a layer inside [`headers_middleware`]
/// already chose the response's security headers.
///
/// `rullst_security::SecureHeadersLayer` inserts it. When the Core baseline
/// finds it, it leaves every security header to that layer, including the ones
/// the layer's configuration omits; it still adds `Cache-Control: no-store`
/// when the response has no cache policy. A custom header layer can insert it
/// for the same effect.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SecurityHeadersApplied;

/// A cryptographically random CSP nonce associated with one request.
///
/// Handlers and renderers can extract this value with `Extension<CspNonce>` and add
/// `nonce="..."` to trusted inline `<script>` or `<style>` elements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CspNonce(String);

impl CspNonce {
    /// Generates a fresh 128-bit nonce using the operating system random source.
    pub fn generate() -> Self {
        let mut bytes = [0_u8; 16];
        rand::rng().fill_bytes(&mut bytes);
        Self(STANDARD.encode(bytes))
    }

    /// Returns the nonce value for an HTML `nonce` attribute.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the nonce already associated with a request or creates and stores a fresh one.
    ///
    /// Security layers should use this helper instead of unconditionally replacing the request
    /// extension so independently composed layers and the renderer share one nonce identity.
    pub fn get_or_insert(extensions: &mut axum::http::Extensions) -> Self {
        if let Some(nonce) = extensions.get::<Self>() {
            return nonce.clone();
        }

        let nonce = Self::generate();
        extensions.insert(nonce.clone());
        nonce
    }
}

impl std::fmt::Display for CspNonce {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Renders a CSP template against an optional request nonce.
///
/// A template containing `{NONCE}` cannot be emitted without a nonce; in that case the strict
/// static policy is used instead of sending a broken or permissive directive.
pub fn render_csp_policy(template: Option<&str>, nonce: Option<&CspNonce>) -> String {
    let template = template.filter(|value| !value.trim().is_empty());
    match (template, nonce) {
        (Some(template), Some(nonce)) => template.replace("{NONCE}", nonce.as_str()),
        (Some(template), None) if !template.contains("{NONCE}") => template.to_owned(),
        (None, Some(nonce)) => DEFAULT_CSP_TEMPLATE.replace("{NONCE}", nonce.as_str()),
        _ => DEFAULT_STATIC_CSP.to_owned(),
    }
}

/// Applies a configured policy without weakening an endpoint's explicit
/// `no-referrer`. Normalizes that restriction to one value even when other
/// header values were appended. Other policies retain the layer's precedence.
pub fn apply_referrer_policy(headers: &mut HeaderMap, configured: HeaderValue) {
    let policy = if headers
        .get_all(header::REFERRER_POLICY)
        .iter()
        .any(|value| value == "no-referrer")
    {
        HeaderValue::from_static("no-referrer")
    } else {
        configured
    };
    headers.insert(header::REFERRER_POLICY, policy);
}

/// Middleware that adds strict security headers and exposes a matching CSP nonce to handlers.
///
/// Each header is added only when the response has none, so an explicit value from a handler
/// or an inner layer wins over this baseline. A response marked with [`SecurityHeadersApplied`]
/// keeps exactly the security headers the inner layer chose.
pub async fn headers_middleware(mut req: Request, next: Next) -> Response {
    let configured_csp = req
        .extensions()
        .get::<crate::config::SecurityConfig>()
        .map(|config| config.csp.clone())
        .unwrap_or_else(|| crate::config::RullstConfig::global().security.csp.clone());
    let configured_coep = req
        .extensions()
        .get::<crate::config::SecurityConfig>()
        .map(|config| config.coep.clone())
        .unwrap_or_else(|| crate::config::RullstConfig::global().security.coep.clone());
    // Reuse a nonce installed by an outer security layer. Generated applications and
    // integrations may compose more than one header layer; replacing the request nonce here
    // would make the renderer use a different value from the final CSP response header.
    let nonce = CspNonce::get_or_insert(req.extensions_mut());

    let mut response = next.run(req).await;
    let inner_layer_applied = response
        .extensions()
        .get::<SecurityHeadersApplied>()
        .is_some();
    let headers = response.headers_mut();

    // Dynamic responses are private by default. A handler can still opt into an explicit,
    // reviewed cache policy for public/static content before this middleware runs.
    headers
        .entry(header::CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("no-store"));
    if inner_layer_applied {
        return response;
    }

    let defaults = [
        (header::X_FRAME_OPTIONS, "DENY"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        // The legacy XSS auditor has caused response mutation vulnerabilities in old browsers.
        (HeaderName::from_static("x-xss-protection"), "0"),
        (
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=63072000; includeSubDomains; preload",
        ),
        (
            HeaderName::from_static("permissions-policy"),
            "camera=(), microphone=(), geolocation=(), payment=(), usb=()",
        ),
        (
            HeaderName::from_static("cross-origin-opener-policy"),
            "same-origin",
        ),
        (
            HeaderName::from_static("cross-origin-resource-policy"),
            "same-origin",
        ),
    ];
    for (name, value) in defaults {
        headers
            .entry(name)
            .or_insert(HeaderValue::from_static(value));
    }
    // An explicit policy wins, but an exact `no-referrer` among several values is still
    // normalized to that single restriction.
    let explicit_no_referrer = headers
        .get_all(header::REFERRER_POLICY)
        .iter()
        .any(|value| value == "no-referrer");
    if explicit_no_referrer || !headers.contains_key(header::REFERRER_POLICY) {
        apply_referrer_policy(
            headers,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        );
    }
    headers
        .entry(HeaderName::from_static("cross-origin-embedder-policy"))
        .or_insert_with(|| {
            HeaderValue::from_str(&configured_coep)
                .unwrap_or_else(|_| HeaderValue::from_static("require-corp"))
        });
    headers
        .entry(header::CONTENT_SECURITY_POLICY)
        .or_insert_with(|| {
            let csp = render_csp_policy(Some(&configured_csp), Some(&nonce));
            HeaderValue::from_str(&csp).unwrap_or_else(|_| {
                HeaderValue::from_str(&render_csp_policy(None, Some(&nonce)))
                    .unwrap_or_else(|_| HeaderValue::from_static("default-src 'none'"))
            })
        });

    response
}
