//! Secure response headers and per-request Content Security Policy nonces.

use axum::{
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::Response,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use rand::Rng;

pub use crate::config::DEFAULT_CSP_TEMPLATE;

const DEFAULT_STATIC_CSP: &str = "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; font-src 'self'; worker-src 'self' blob:";

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

/// Middleware that injects strict security headers and exposes a matching CSP nonce to handlers.
pub async fn headers_middleware(mut req: Request, next: Next) -> Response {
    let configured_csp = req
        .extensions()
        .get::<crate::config::SecurityConfig>()
        .map(|config| config.csp.clone())
        .unwrap_or_else(|| crate::config::RullstConfig::global().security.csp.clone());
    // Reuse a nonce installed by an outer security layer. Generated applications and
    // integrations may compose more than one header layer; replacing the request nonce here
    // would make the renderer use a different value from the final CSP response header.
    let nonce = CspNonce::get_or_insert(req.extensions_mut());

    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    // The legacy XSS auditor has caused response mutation vulnerabilities in old browsers.
    headers.insert("x-xss-protection", HeaderValue::from_static("0"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
    );
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=(), usb=()"),
    );
    headers.insert(
        "cross-origin-opener-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        "cross-origin-embedder-policy",
        HeaderValue::from_static("require-corp"),
    );

    let csp = render_csp_policy(Some(&configured_csp), Some(&nonce));
    let csp_value = HeaderValue::from_str(&csp).unwrap_or_else(|_| {
        HeaderValue::from_str(&render_csp_policy(None, Some(&nonce)))
            .unwrap_or_else(|_| HeaderValue::from_static("default-src 'none'"))
    });
    headers.insert(header::CONTENT_SECURITY_POLICY, csp_value);

    response
}
