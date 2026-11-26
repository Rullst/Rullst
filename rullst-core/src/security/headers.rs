//! Secure response headers and per-request Content Security Policy nonces.

use axum::{
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue, header},
    middleware::Next,
    response::Response,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use rand::Rng;
use std::sync::{Arc, OnceLock};

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
///
/// The value is 128 bits from a CSPRNG (`rand`'s thread-local ChaCha12
/// generator, seeded and reseeded from the operating system's random source),
/// in standard base64. The nonce the security layers attach to a request
/// draws it on first use: when a handler or renderer reads it, or when the
/// header layer needs it for the policy of an HTML response. A request that
/// never reads it and does not answer with a document draws nothing. Clones
/// share one value, so the page and its `Content-Security-Policy` header
/// always agree, and every request gets a value of its own.
#[derive(Clone)]
pub struct CspNonce(Arc<OnceLock<String>>);

/// A fresh nonce value: 16 bytes from the thread's CSPRNG in base64.
fn draw_nonce() -> String {
    let mut bytes = [0_u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    STANDARD.encode(bytes)
}

impl CspNonce {
    /// Generates a fresh 128-bit nonce now, from the CSPRNG described on
    /// [`CspNonce`].
    pub fn generate() -> Self {
        Self(Arc::new(OnceLock::from(draw_nonce())))
    }

    /// A nonce whose value is drawn the first time any clone reads it.
    fn deferred() -> Self {
        Self(Arc::new(OnceLock::new()))
    }

    /// Returns the nonce value for an HTML `nonce` attribute, drawing it on
    /// the first call.
    pub fn as_str(&self) -> &str {
        self.0.get_or_init(draw_nonce)
    }

    /// Whether the value was drawn: always for [`CspNonce::generate`], and
    /// for the nonce of a request once anything read it.
    #[cfg(test)]
    pub(crate) fn is_drawn(&self) -> bool {
        self.0.get().is_some()
    }

    /// Returns the nonce already associated with a request or creates and stores a fresh one.
    ///
    /// Security layers should use this helper instead of unconditionally replacing the request
    /// extension so independently composed layers and the renderer share one nonce identity.
    /// The stored nonce draws its value on first use (see [`CspNonce`]); the returned clone
    /// shares it.
    pub fn get_or_insert(extensions: &mut axum::http::Extensions) -> Self {
        if let Some(nonce) = extensions.get::<Self>() {
            return nonce.clone();
        }

        let nonce = Self::deferred();
        extensions.insert(nonce.clone());
        nonce
    }
}

impl PartialEq for CspNonce {
    /// Nonces are equal when their values are, drawing both.
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for CspNonce {}

impl std::fmt::Debug for CspNonce {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("CspNonce")
            .field(&self.as_str())
            .finish()
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

/// The `Strict-Transport-Security` value that `config` selects, for example
/// `max-age=31536000; includeSubDomains` with the defaults.
fn strict_transport_security(config: &crate::config::SecurityConfig) -> String {
    let mut value = format!("max-age={}", config.hsts_max_age_seconds);
    if config.hsts_include_subdomains {
        value.push_str("; includeSubDomains");
    }
    if config.hsts_preload {
        value.push_str("; preload");
    }
    value
}

const PERMISSIONS_POLICY: HeaderName = HeaderName::from_static("permissions-policy");
const CROSS_ORIGIN_OPENER_POLICY: HeaderName =
    HeaderName::from_static("cross-origin-opener-policy");
const CROSS_ORIGIN_RESOURCE_POLICY: HeaderName =
    HeaderName::from_static("cross-origin-resource-policy");
const CROSS_ORIGIN_EMBEDDER_POLICY: HeaderName =
    HeaderName::from_static("cross-origin-embedder-policy");

/// The fixed headers of the baseline, each added when the response has none.
const DEFAULT_HEADERS: [(HeaderName, HeaderValue); 6] = [
    (header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")),
    (
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ),
    // The legacy XSS auditor has caused response mutation vulnerabilities in old browsers.
    (header::X_XSS_PROTECTION, HeaderValue::from_static("0")),
    (
        PERMISSIONS_POLICY,
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=(), usb=()"),
    ),
    (
        CROSS_ORIGIN_OPENER_POLICY,
        HeaderValue::from_static("same-origin"),
    ),
    (
        CROSS_ORIGIN_RESOURCE_POLICY,
        HeaderValue::from_static("same-origin"),
    ),
];
/// Every header [`with_security_headers`] can add: `Cache-Control`, then
/// those of [`HeaderPlan::apply`] in its order.
const STEP_HEADERS: [HeaderName; 11] = [
    header::CACHE_CONTROL,
    header::X_FRAME_OPTIONS,
    header::X_CONTENT_TYPE_OPTIONS,
    header::X_XSS_PROTECTION,
    PERMISSIONS_POLICY,
    CROSS_ORIGIN_OPENER_POLICY,
    CROSS_ORIGIN_RESOURCE_POLICY,
    header::STRICT_TRANSPORT_SECURITY,
    header::REFERRER_POLICY,
    CROSS_ORIGIN_EMBEDDER_POLICY,
    header::CONTENT_SECURITY_POLICY,
];
/// The most header names a response may have for [`HeaderPlan::append_all`]:
/// comparing them with [`STEP_HEADERS`] then costs less than the lookups it
/// saves.
const SCANNED_NAMES: usize = 8;
const NO_STORE: HeaderValue = HeaderValue::from_static("no-store");
/// The headers [`HeaderPlan::apply`] can add: the defaults, HSTS,
/// `Referrer-Policy`, COEP and CSP.
const PLANNED_HEADERS: usize = 10;
const STRICT_ORIGIN_WHEN_CROSS_ORIGIN: HeaderValue =
    HeaderValue::from_static("strict-origin-when-cross-origin");

/// The header values [`headers_middleware`] derives from a
/// [`crate::config::SecurityConfig`], built once per configuration instead of
/// for every response.
#[derive(Clone, Debug)]
pub(crate) struct HeaderPlan {
    /// The template each response's policy is rendered from, split at its
    /// `{NONCE}` placeholders: the configured `csp`, or
    /// [`DEFAULT_CSP_TEMPLATE`] when that is blank or holds a byte a header
    /// value cannot carry.
    csp_parts: Vec<String>,
    /// The policy of a response that cannot be an HTML document: the
    /// template when it has no `{NONCE}`, else [`DEFAULT_STATIC_CSP`]
    /// ([`render_csp_policy`] without a nonce).
    static_csp: HeaderValue,
    coep: HeaderValue,
    hsts: Option<HeaderValue>,
}

impl HeaderPlan {
    pub(crate) fn new(config: &crate::config::SecurityConfig) -> Self {
        // A nonce is base64 text, so the rendered policy is a valid header
        // value exactly when the template is.
        let usable = !config.csp.trim().is_empty() && HeaderValue::from_str(&config.csp).is_ok();
        let template = if usable {
            config.csp.as_str()
        } else {
            DEFAULT_CSP_TEMPLATE
        };
        let static_csp = if template.contains("{NONCE}") {
            DEFAULT_STATIC_CSP
        } else {
            template
        };
        Self {
            csp_parts: template.split("{NONCE}").map(str::to_owned).collect(),
            static_csp: HeaderValue::from_str(static_csp)
                .unwrap_or_else(|_| HeaderValue::from_static(DEFAULT_STATIC_CSP)),
            coep: HeaderValue::from_str(&config.coep)
                .unwrap_or_else(|_| HeaderValue::from_static("require-corp")),
            hsts: HeaderValue::from_str(&strict_transport_security(config)).ok(),
        }
    }

    /// This plan without `Strict-Transport-Security`.
    pub(crate) fn without_hsts(&self) -> Self {
        Self {
            hsts: None,
            ..self.clone()
        }
    }

    /// The `Content-Security-Policy` value for one request's nonce.
    fn csp(&self, nonce: &CspNonce) -> HeaderValue {
        let policy = self.csp_parts.join(nonce.as_str());
        HeaderValue::from_maybe_shared(axum::body::Bytes::from(policy))
            .unwrap_or_else(|_| HeaderValue::from_static("default-src 'none'"))
    }

    /// The `Content-Security-Policy` of a response: with `nonce` when it may
    /// be a document.
    fn policy(&self, document: bool, nonce: &CspNonce) -> HeaderValue {
        // Only a document can run the inline scripts and styles a nonce
        // admits; any other response gets the fixed policy without one.
        if document {
            self.csp(nonce)
        } else {
            self.static_csp.clone()
        }
    }

    /// `Cache-Control: no-store` and every header of the plan, for a
    /// response that has none of [`STEP_HEADERS`] ([`lacks_step_headers`]):
    /// what the default and [`Self::apply`] add to it, in the same order,
    /// appended without looking each one up first.
    fn append_all(&self, headers: &mut HeaderMap, nonce: &CspNonce) {
        let document = may_be_html(headers);
        make_room(headers, STEP_HEADERS.len());
        headers.append(header::CACHE_CONTROL, NO_STORE);
        for (name, value) in DEFAULT_HEADERS {
            headers.append(name, value);
        }
        if let Some(hsts) = &self.hsts {
            headers.append(header::STRICT_TRANSPORT_SECURITY, hsts.clone());
        }
        headers.append(header::REFERRER_POLICY, STRICT_ORIGIN_WHEN_CROSS_ORIGIN);
        headers.append(CROSS_ORIGIN_EMBEDDER_POLICY, self.coep.clone());
        headers.append(
            header::CONTENT_SECURITY_POLICY,
            self.policy(document, nonce),
        );
    }

    /// Adds every header of the plan that `headers` lacks.
    fn apply(&self, headers: &mut HeaderMap, nonce: &CspNonce) {
        make_room(headers, PLANNED_HEADERS);
        for (name, value) in DEFAULT_HEADERS {
            headers.entry(name).or_insert(value);
        }
        if let Some(hsts) = &self.hsts {
            headers
                .entry(header::STRICT_TRANSPORT_SECURITY)
                .or_insert_with(|| hsts.clone());
        }
        // An explicit policy wins, but an exact `no-referrer` among several values is still
        // normalized to that single restriction.
        let explicit_no_referrer = headers
            .get_all(header::REFERRER_POLICY)
            .iter()
            .any(|value| value == "no-referrer");
        if explicit_no_referrer || !headers.contains_key(header::REFERRER_POLICY) {
            apply_referrer_policy(headers, STRICT_ORIGIN_WHEN_CROSS_ORIGIN);
        }
        headers
            .entry(CROSS_ORIGIN_EMBEDDER_POLICY)
            .or_insert_with(|| self.coep.clone());
        let document = may_be_html(headers);
        headers
            .entry(header::CONTENT_SECURITY_POLICY)
            .or_insert_with(|| self.policy(document, nonce));
    }
}

/// Whether a response with `headers` has none of [`STEP_HEADERS`], found by
/// comparing its names when it has at most [`SCANNED_NAMES`]; with more,
/// `false`, and [`HeaderPlan::apply`] looks each header up instead.
pub(super) fn lacks_step_headers(headers: &HeaderMap) -> bool {
    headers.keys_len() <= SCANNED_NAMES && !headers.keys().any(|name| STEP_HEADERS.contains(name))
}

/// Whether a response with `headers` can be an HTML document: its
/// `Content-Type` is `text/html` or `application/xhtml+xml`, or it has no
/// readable one, which a browser may sniff.
fn may_be_html(headers: &HeaderMap) -> bool {
    match headers.get(header::CONTENT_TYPE).map(HeaderValue::to_str) {
        Some(Ok(content_type)) => {
            let media_type = super::media_type::essence(content_type);
            media_type.eq_ignore_ascii_case("text/html")
                || media_type.eq_ignore_ascii_case("application/xhtml+xml")
        }
        _ => true,
    }
}

/// Grows `headers` once for the `additional` headers of a plan, instead of
/// several times while they are added. Capacity only: a map that cannot grow
/// keeps its size, and the headers are the same either way.
#[cfg_attr(mutants, mutants::skip)]
fn make_room(headers: &mut HeaderMap, additional: usize) {
    if headers.capacity() < headers.len() + additional {
        let _ = headers.try_reserve(additional);
    }
}

/// The configuration a [`PlanCache`] holds the plan of.
#[derive(Debug)]
struct PlanSource {
    csp: String,
    coep: String,
    hsts: (u64, bool, bool),
}

impl PlanSource {
    fn of(config: &crate::config::SecurityConfig) -> Self {
        Self {
            csp: config.csp.clone(),
            coep: config.coep.clone(),
            hsts: hsts_settings(config),
        }
    }

    fn matches(&self, config: &crate::config::SecurityConfig) -> bool {
        (self.csp.as_str(), self.coep.as_str(), self.hsts)
            == (
                config.csp.as_str(),
                config.coep.as_str(),
                hsts_settings(config),
            )
    }
}

fn hsts_settings(config: &crate::config::SecurityConfig) -> (u64, bool, bool) {
    (
        config.hsts_max_age_seconds,
        config.hsts_include_subdomains,
        config.hsts_preload,
    )
}

/// The plan of the first configuration it sees. An application has one
/// security configuration, so every later request borrows that plan; another
/// configuration gets a plan of its own for each request.
#[derive(Debug)]
pub(crate) struct PlanCache(std::sync::OnceLock<(PlanSource, HeaderPlan)>);

impl PlanCache {
    pub(crate) const fn new() -> Self {
        Self(std::sync::OnceLock::new())
    }

    pub(super) fn plan(
        &self,
        config: &crate::config::SecurityConfig,
    ) -> std::borrow::Cow<'_, HeaderPlan> {
        let (source, plan) = self
            .0
            .get_or_init(|| (PlanSource::of(config), HeaderPlan::new(config)));
        if source.matches(config) {
            std::borrow::Cow::Borrowed(plan)
        } else {
            std::borrow::Cow::Owned(HeaderPlan::new(config))
        }
    }
}

static PLANS: PlanCache = PlanCache::new();

/// Middleware that adds strict security headers and exposes a matching CSP nonce to handlers.
///
/// Each header is added only when the response has none, so an explicit value from a handler
/// or an inner layer wins over this baseline. A response marked with [`SecurityHeadersApplied`]
/// keeps exactly the security headers the inner layer chose.
///
/// `Strict-Transport-Security` comes from the `hsts_max_age_seconds`,
/// `hsts_include_subdomains` and `hsts_preload` settings of
/// [`crate::config::SecurityConfig`] (`[security]` in `Rullst.toml`); the
/// default is `max-age=31536000; includeSubDomains`, without `preload`.
pub async fn headers_middleware(req: Request, next: Next) -> Response {
    let plan = PLANS.plan(super::waf::security_config(&req));
    with_security_headers(req, &plan, |req| next.run(req)).await
}

/// [`headers_middleware`] with the header values of `plan`, around `next`.
pub(crate) async fn with_security_headers<F, Fut>(
    mut req: Request,
    plan: &HeaderPlan,
    next: F,
) -> Response
where
    F: FnOnce(Request) -> Fut,
    Fut: std::future::Future<Output = Response>,
{
    // Reuse a nonce installed by an outer security layer. Generated applications and
    // integrations may compose more than one header layer; replacing the request nonce here
    // would make the renderer use a different value from the final CSP response header.
    // Its value is drawn only if the handler reads it or the response is a document.
    let nonce = CspNonce::get_or_insert(req.extensions_mut());

    let mut response = next(req).await;
    let inner_layer_applied = response
        .extensions()
        .get::<SecurityHeadersApplied>()
        .is_some();
    let headers = response.headers_mut();
    if !inner_layer_applied && lacks_step_headers(headers) {
        plan.append_all(headers, &nonce);
        return response;
    }

    // Dynamic responses are private by default. A handler can still opt into an explicit,
    // reviewed cache policy for public/static content before this middleware runs.
    headers.entry(header::CACHE_CONTROL).or_insert(NO_STORE);
    if !inner_layer_applied {
        plan.apply(headers, &nonce);
    }
    response
}
