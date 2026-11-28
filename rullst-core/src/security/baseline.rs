//! Canonical composition of the runtime-owned browser security baseline.

use crate::config::{Environment, SecurityConfig};
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum::{Router, http};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;

/// Applies the full baseline and an exact authenticated machine-route policy.
/// The baseline's CSRF layer authenticates the policy's routes in every
/// environment; the legacy signed-webhook path list is unchanged.
pub fn apply_security_baseline_with_machine_endpoints(
    app: Router,
    security: SecurityConfig,
    environment: Environment,
    policy: super::MachineEndpointPolicy,
) -> Result<Router, SecurityBaselineError> {
    Ok(apply_security_baseline(app, security, environment)?.layer(axum::Extension(policy)))
}

/// Failure to construct the runtime security baseline from application configuration.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum SecurityBaselineError {
    /// A configured CORS origin cannot be represented as an HTTP header value.
    #[error("invalid CORS origin header value `{0}`")]
    InvalidCorsOrigin(String),
    /// A cross-field security configuration invariant failed.
    #[error("invalid security baseline configuration: {0}")]
    InvalidConfiguration(String),
}

/// Applies the canonical runtime-owned browser security baseline.
///
/// The outer-to-inner order is secure headers → CORS → secure framework
/// cookies → WAF → CSRF → optional response PII masking → handler. They run
/// as one middleware (two around a CORS layer, plus PII masking when
/// enabled), with the header values derived once from `security`.
///
/// Secure headers and CSRF run in every environment since 13.0.0-alpha.6
/// (before, only in staging and production), so development behaves like
/// production: a state-changing request needs the CSRF token, and handlers
/// find [`super::CsrfToken`] and [`super::CspNonce`]. In development and test
/// `Strict-Transport-Security` is sent only to a request that arrived over
/// HTTPS, so a plain-HTTP `localhost` is never pinned. The WAF refuses
/// requests in staging and production; in development it only logs, at
/// `warn` level, the rule and path of each request it would refuse, and in
/// test it does not run. PII masking runs in staging and production when
/// `enable_pii_masking` is set.
///
/// In every environment, the secure framework cookies layer adds `Secure` to
/// the `rullst_*` cookies (session, CSRF) of a response to a request that
/// arrived over HTTPS: an `https` request URI, or `X-Forwarded-Proto: https`
/// accepted by [`super::TrustedProxyLayer`] from a trusted proxy with
/// `trust_forwarded_proto`; the CSRF cookie carries `Secure` in staging and
/// production anyway. Configuration and environment extensions are installed
/// outside those layers so every middleware observes the exact application
/// configuration rather than a process-global fallback. Session,
/// authentication, tenancy and authorization remain application-owned layers.
pub fn apply_security_baseline(
    app: Router,
    security: SecurityConfig,
    environment: Environment,
) -> Result<Router, SecurityBaselineError> {
    apply_server_baseline(app, security, environment, false)
}

/// [`apply_security_baseline`] for [`crate::Server`]'s stack. With
/// `vary_html`, the baseline also adds `Vary: HX-Request` to the HTML
/// responses of `app`'s routes, right after them, where the server's
/// separate `Vary` layer would; the server asks for it only when no layer
/// between the baseline and the routes answers or changes a response.
pub(crate) fn apply_server_baseline(
    mut app: Router,
    security: SecurityConfig,
    environment: Environment,
    vary_html: bool,
) -> Result<Router, SecurityBaselineError> {
    security
        .validate()
        .map_err(|error| SecurityBaselineError::InvalidConfiguration(error.to_string()))?;
    let cors = cors_layer(&security)?;

    if masks_pii(&security, environment) {
        app = app.layer(axum::middleware::from_fn(
            crate::security::pii_masking_middleware,
        ));
    }
    let headers = super::headers::HeaderPlan::new(&security);
    let baseline = Arc::new(Baseline {
        headers_over_http: if environment.requires_secure_defaults() {
            headers.clone()
        } else {
            headers.without_hsts()
        },
        headers,
        waf: if environment.requires_secure_defaults() {
            Waf::Enforce
        } else if environment.allows_development_tools() {
            Waf::Report
        } else {
            Waf::Off
        },
        security,
        environment,
    });
    Ok(if vary_html {
        compose::<Varying>(app, baseline, cors)
    } else {
        compose::<Plain>(app, baseline, cors)
    })
}

/// `app` inside the baseline's middleware, with `S` inside its checks.
fn compose<S: RouteStep>(app: Router, baseline: Arc<Baseline>, cors: Option<CorsLayer>) -> Router {
    // Only a CORS policy splits the baseline, to keep its place.
    match cors {
        None => app.layer(axum::middleware::from_fn_with_state(
            baseline,
            whole_baseline::<S>,
        )),
        Some(cors) => app
            .layer(axum::middleware::from_fn_with_state(
                baseline.clone(),
                request_checks::<S>,
            ))
            .layer(cors)
            .layer(axum::middleware::from_fn_with_state(
                baseline,
                headers_layer,
            )),
    }
}

/// Whether the baseline masks PII in responses: in staging and production,
/// when `enable_pii_masking` is set.
pub(crate) fn masks_pii(security: &SecurityConfig, environment: Environment) -> bool {
    environment.requires_secure_defaults() && security.enable_pii_masking
}

/// The CORS layer of `security.cors_allow_origins`, if it lists any origin.
fn cors_layer(security: &SecurityConfig) -> Result<Option<CorsLayer>, SecurityBaselineError> {
    if security.cors_allow_origins.is_empty() {
        return Ok(None);
    }
    let origins = security
        .cors_allow_origins
        .iter()
        .map(|origin| {
            origin
                .parse::<http::HeaderValue>()
                .map_err(|_| SecurityBaselineError::InvalidCorsOrigin(origin.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([
                http::Method::GET,
                http::Method::HEAD,
                http::Method::POST,
                http::Method::PUT,
                http::Method::PATCH,
                http::Method::DELETE,
            ])
            .allow_headers([
                http::header::ACCEPT,
                http::header::AUTHORIZATION,
                http::header::CONTENT_TYPE,
                http::HeaderName::from_static("x-csrf-token"),
            ])
            .allow_credentials(security.cors_allow_credentials)
            .max_age(Duration::from_secs(600)),
    ))
}

/// How the baseline's WAF treats a request it would refuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Waf {
    /// Staging and production: refuse it.
    Enforce,
    /// Development: log it and let it through.
    Report,
    /// Test: no inspection.
    Off,
}

/// What the baseline needs for every request.
struct Baseline {
    security: SecurityConfig,
    environment: Environment,
    headers: super::headers::HeaderPlan,
    /// The plan for a plain-HTTP request: without HSTS outside staging and
    /// production.
    headers_over_http: super::headers::HeaderPlan,
    waf: Waf,
}

/// What runs inside the baseline's checks: the routes as they are
/// ([`Plain`]), or the routes and then the `Vary: HX-Request` step
/// ([`Varying`], see [`apply_server_baseline`]).
trait RouteStep: Send + Sync + 'static {
    type Routes: super::Run;
    fn routes(next: Next) -> Self::Routes;
}

/// The routes as they are.
struct Plain;

impl RouteStep for Plain {
    type Routes = Next;
    fn routes(next: Next) -> Next {
        next
    }
}

/// The routes, then `Vary: HX-Request` on their HTML responses.
struct Varying;

impl RouteStep for Varying {
    type Routes = VaryingRoutes;
    fn routes(next: Next) -> VaryingRoutes {
        VaryingRoutes(next)
    }
}

struct VaryingRoutes(Next);

impl super::Run for VaryingRoutes {
    async fn run(self, req: Request) -> Response {
        let mut response = self.0.run(req).await;
        crate::htmx::vary_route_response(&mut response);
        response
    }
}

/// The baseline without a CORS policy, outer to inner: configuration
/// extensions, secure headers, secure framework cookies, WAF, CSRF.
async fn whole_baseline<S: RouteStep>(
    State(baseline): State<Arc<Baseline>>,
    req: Request,
    next: Next,
) -> Response {
    let req = baseline.install(req);
    let plan = baseline.header_plan(&req);
    super::headers::with_security_headers(req, plan, |req| {
        with_request_checks(req, baseline.waf, S::routes(next))
    })
    .await
}

/// The part of the baseline outside a CORS policy: configuration extensions
/// and secure headers.
async fn headers_layer(
    State(baseline): State<Arc<Baseline>>,
    req: Request,
    next: Next,
) -> Response {
    let req = baseline.install(req);
    let plan = baseline.header_plan(&req);
    super::headers::with_security_headers(req, plan, |req| next.run(req)).await
}

/// The part of the baseline inside a CORS policy: secure framework cookies
/// (outside CSRF, so the cookie it mints is covered too), WAF, CSRF.
async fn request_checks<S: RouteStep>(
    State(baseline): State<Arc<Baseline>>,
    req: Request,
    next: Next,
) -> Response {
    with_request_checks(req, baseline.waf, S::routes(next)).await
}

async fn with_request_checks(req: Request, waf: Waf, next: impl super::Run) -> Response {
    super::secure_cookies::with_secure_framework_cookies(req, |req| inspected(req, waf, next)).await
}

async fn inspected(req: Request, waf: Waf, next: impl super::Run) -> Response {
    match waf {
        Waf::Enforce => super::waf::with_waf(req, |req| super::csrf::with_csrf(req, next)).await,
        Waf::Report => {
            super::waf::with_waf_report(req, |req| super::csrf::with_csrf(req, next)).await
        }
        Waf::Off => super::csrf::with_csrf(req, next).await,
    }
}

impl Baseline {
    /// Installs the configuration and environment extensions, so every
    /// middleware observes the exact application configuration rather than a
    /// process-global fallback.
    fn install(&self, mut req: Request) -> Request {
        req.extensions_mut().insert(self.security.clone());
        req.extensions_mut().insert(self.environment);
        req
    }

    /// The header values for `req`: HSTS only over HTTPS outside staging and
    /// production.
    fn header_plan(&self, req: &Request) -> &super::headers::HeaderPlan {
        if super::secure_cookies::request_is_https(req.uri(), req.extensions()) {
            &self.headers
        } else {
            &self.headers_over_http
        }
    }
}

#[cfg(test)]
#[path = "baseline_environment_tests.rs"]
mod environment_tests;

#[cfg(test)]
#[path = "baseline_tests.rs"]
mod tests;
