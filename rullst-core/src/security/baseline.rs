//! Canonical composition of the runtime-owned browser security baseline.

use crate::config::{Environment, SecurityConfig};
use axum::{Router, http};
use std::time::Duration;
use tower_http::cors::CorsLayer;

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
/// In staging/production the outer-to-inner order is secure headers → CORS →
/// WAF/RASP → CSRF → optional response PII masking → handler. Configuration and
/// environment extensions are installed outside those layers so every
/// middleware observes the exact application configuration rather than a
/// process-global fallback. Session, authentication, tenancy and authorization
/// remain application-owned layers.
pub fn apply_security_baseline(
    mut app: Router,
    security: SecurityConfig,
    environment: Environment,
) -> Result<Router, SecurityBaselineError> {
    security
        .validate()
        .map_err(|error| SecurityBaselineError::InvalidConfiguration(error.to_string()))?;

    if environment.requires_secure_defaults() {
        if security.enable_pii_masking {
            app = app.layer(axum::middleware::from_fn(
                crate::security::pii_masking_middleware,
            ));
        }
        app = app
            .layer(axum::middleware::from_fn(crate::security::csrf_middleware))
            .layer(axum::middleware::from_fn(crate::security::waf_middleware));
    }

    if !security.cors_allow_origins.is_empty() {
        let origins = security
            .cors_allow_origins
            .iter()
            .map(|origin| {
                origin
                    .parse::<http::HeaderValue>()
                    .map_err(|_| SecurityBaselineError::InvalidCorsOrigin(origin.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        app = app.layer(
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
        );
    }

    if environment.requires_secure_defaults() {
        app = app.layer(axum::middleware::from_fn(
            crate::security::headers_middleware,
        ));
    }

    Ok(app
        .layer(axum::Extension(security))
        .layer(axum::Extension(environment)))
}

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
                        format!(
                            "{}|{}|{}",
                            nonce.as_str(),
                            csrf.as_str(),
                            config.csrf_same_site
                        )
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
        assert!(cookie.starts_with(&format!("rullst_csrf={csrf};")));
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
                    .header(header::COOKIE, format!("rullst_csrf={csrf}"))
                    .header("x-csrf-token", csrf)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(accepted.status(), StatusCode::NO_CONTENT);
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
