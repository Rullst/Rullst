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
    mut app: Router,
    security: SecurityConfig,
    environment: Environment,
) -> Result<Router, SecurityBaselineError> {
    security
        .validate()
        .map_err(|error| SecurityBaselineError::InvalidConfiguration(error.to_string()))?;
    let cors = cors_layer(&security)?;

    if environment.requires_secure_defaults() && security.enable_pii_masking {
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
    // Only a CORS policy splits the baseline, to keep its place.
    Ok(match cors {
        None => app.layer(axum::middleware::from_fn_with_state(
            baseline,
            whole_baseline,
        )),
        Some(cors) => app
            .layer(axum::middleware::from_fn_with_state(
                baseline.clone(),
                request_checks,
            ))
            .layer(cors)
            .layer(axum::middleware::from_fn_with_state(
                baseline,
                headers_layer,
            )),
    })
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

/// The baseline without a CORS policy, outer to inner: configuration
/// extensions, secure headers, secure framework cookies, WAF, CSRF.
async fn whole_baseline(
    State(baseline): State<Arc<Baseline>>,
    req: Request,
    next: Next,
) -> Response {
    let req = baseline.install(req);
    let plan = baseline.header_plan(&req);
    super::headers::with_security_headers(req, plan, |req| {
        with_request_checks(req, baseline.waf, next)
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
async fn request_checks(
    State(baseline): State<Arc<Baseline>>,
    req: Request,
    next: Next,
) -> Response {
    with_request_checks(req, baseline.waf, next).await
}

async fn with_request_checks(req: Request, waf: Waf, next: Next) -> Response {
    super::secure_cookies::with_secure_framework_cookies(req, |req| inspected(req, waf, next)).await
}

async fn inspected(req: Request, waf: Waf, next: Next) -> Response {
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
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::field_reassign_with_default
)]
mod tests {
    use super::*;
    use crate::security::{CspNonce, CsrfToken};
    use axum::{
        body::{Body, to_bytes},
        extract::Extension,
        http::{Request, StatusCode, header},
        routing::{get, post},
    };
    use tower::ServiceExt;

    fn production_app() -> Router {
        let mut security = SecurityConfig::default();
        security.csrf_same_site = "Strict".to_string();
        security.cors_allow_origins = vec!["https://academy.example".to_string()];
        security.cors_allow_credentials = true;
        let app = Router::new()
            .route(
                "/",
                get(
                    |Extension(nonce): Extension<CspNonce>,
                     Extension(csrf): Extension<CsrfToken>,
                     Extension(config): Extension<SecurityConfig>| async move {
                        // A page: only a document's policy carries the nonce.
                        axum::response::Html(format!(
                            "{}|{}|{}",
                            nonce.as_str(),
                            csrf.as_str(),
                            config.csrf_same_site
                        ))
                    },
                ),
            )
            .route("/write", post(|| async { StatusCode::NO_CONTENT }));
        apply_security_baseline(app, security, Environment::Production)
            .expect("production security baseline")
    }

    fn browser_request(method: http::Method, path: &str) -> http::request::Builder {
        Request::builder()
            .method(method)
            .uri(path)
            .header(header::USER_AGENT, "Mozilla/5.0")
            .header(header::ORIGIN, "https://academy.example")
    }

    #[tokio::test]
    async fn production_baseline_composes_nonce_cors_csrf_and_headers() {
        // TM-NEXUS-05: a cookie-authenticated write is denied before the handler
        // unless the exact double-submit token accompanies the request.
        let app = production_app();
        let response = app
            .clone()
            .oneshot(
                browser_request(http::Method::GET, "/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&http::HeaderValue::from_static("https://academy.example")),
        );
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS),
            Some(&http::HeaderValue::from_static("true")),
        );
        assert_eq!(
            response.headers().get(header::X_FRAME_OPTIONS),
            Some(&http::HeaderValue::from_static("DENY")),
        );
        let csp = response
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("CSP header")
            .to_str()
            .expect("CSP text")
            .to_string();
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .expect("CSRF cookie")
            .to_str()
            .expect("CSRF cookie text")
            .to_string();
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let body = std::str::from_utf8(&body).expect("baseline response text");
        let mut fields = body.split('|');
        let nonce = fields.next().expect("nonce field");
        let csrf = fields.next().expect("CSRF field");
        assert_eq!(fields.next(), Some("Strict"));
        assert!(csp.contains(&format!("'nonce-{nonce}'")));
        // The page gets the cookie token masked for this response.
        let cookie_token = cookie
            .strip_prefix("rullst_csrf=")
            .and_then(|rest| rest.split(';').next())
            .expect("CSRF cookie token");
        assert_eq!((cookie_token.len(), csrf.len()), (32, 86));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("; Secure"));

        let denied = app
            .clone()
            .oneshot(
                browser_request(http::Method::POST, "/write")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        assert!(
            denied
                .headers()
                .contains_key(header::CONTENT_SECURITY_POLICY)
        );

        let accepted = app
            .oneshot(
                browser_request(http::Method::POST, "/write")
                    .header(header::COOKIE, format!("rullst_csrf={cookie_token}"))
                    .header("x-csrf-token", csrf)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(accepted.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn production_baseline_without_cors_runs_every_layer() {
        let app = Router::new()
            .route(
                "/",
                get(
                    |Extension(nonce): Extension<CspNonce>,
                     Extension(csrf): Extension<CsrfToken>,
                     Extension(config): Extension<SecurityConfig>,
                     Extension(environment): Extension<Environment>| async move {
                        axum::response::Html(format!(
                            "{}|{}|{}|{environment:?}",
                            nonce.as_str(),
                            csrf.as_str(),
                            config.csrf_same_site
                        ))
                    },
                ),
            )
            .route("/write", post(|| async { StatusCode::NO_CONTENT }));
        let app = apply_security_baseline(app, SecurityConfig::default(), Environment::Staging)
            .expect("staging security baseline");
        let get_page = |agent: &str| {
            Request::get("/")
                .header(header::USER_AGENT, agent)
                .body(Body::empty())
                .unwrap()
        };

        let response = app.clone().oneshot(get_page("Mozilla/5.0")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::X_FRAME_OPTIONS], "DENY");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(
            !response
                .headers()
                .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        );
        let csp = response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .to_string();
        let cookie = response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .to_string();
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let body = std::str::from_utf8(&body).unwrap().to_string();
        let fields: Vec<&str> = body.split('|').collect();
        assert_eq!(fields[2..], ["Lax", "Staging"]);
        assert!(csp.contains(&format!("'nonce-{}'", fields[0])), "{csp}");
        // The page's token may be a masked form of the cookie's.
        assert!(!fields[1].is_empty());
        assert!(cookie.starts_with("rullst_csrf="), "{cookie}");
        assert!(
            cookie.ends_with("; Path=/; SameSite=Lax; Secure"),
            "{cookie}"
        );

        // The WAF and CSRF refusals still carry the secure headers.
        for request in [
            get_page("GPTBot/1.0"),
            Request::post("/write").body(Body::empty()).unwrap(),
        ] {
            let refused = app.clone().oneshot(request).await.unwrap();
            assert_eq!(refused.status(), StatusCode::FORBIDDEN);
            assert!(
                refused
                    .headers()
                    .contains_key(header::CONTENT_SECURITY_POLICY)
            );
            assert_eq!(refused.headers()[header::X_FRAME_OPTIONS], "DENY");
        }
    }

    #[tokio::test]
    async fn cors_preflight_is_exact_and_unlisted_origins_receive_no_grant() {
        let app = production_app();
        let preflight = app
            .clone()
            .oneshot(
                browser_request(http::Method::OPTIONS, "/write")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "x-csrf-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preflight.status(), StatusCode::OK);
        assert_eq!(
            preflight.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&http::HeaderValue::from_static("https://academy.example")),
        );

        let unlisted = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(header::USER_AGENT, "Mozilla/5.0")
                    .header(header::ORIGIN, "https://evil.example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unlisted.status(), StatusCode::OK);
        assert!(
            !unlisted
                .headers()
                .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        );
    }
}
