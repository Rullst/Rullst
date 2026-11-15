//! The baseline outside staging and production: CSRF and secure headers in
//! every environment, HSTS only over HTTPS.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::security::{CspNonce, CsrfToken};
use axum::body::Body;
use axum::extract::Extension;
use axum::http::{Request, StatusCode, header};
use axum::routing::{get, post};
use tower::ServiceExt;

const TOKEN: &str = "environment-tests-csrf-token";

fn app(environment: Environment) -> Router {
    let app =
        Router::new()
            .route(
                "/",
                get(
                    |Extension(nonce): Extension<CspNonce>,
                     Extension(csrf): Extension<CsrfToken>| async move {
                        axum::response::Html(format!("{}|{}", nonce.as_str(), csrf.as_str()))
                    },
                ),
            )
            .route("/write", post(|| async { StatusCode::NO_CONTENT }));
    apply_security_baseline(app, SecurityConfig::default(), environment).unwrap()
}

async fn send(app: &Router, request: Request<Body>) -> axum::response::Response {
    app.clone().oneshot(request).await.unwrap()
}

#[tokio::test]
async fn development_and_test_require_the_csrf_token_and_send_secure_headers() {
    for environment in [Environment::Development, Environment::Test] {
        let app = app(environment);
        let page = send(&app, Request::get("/").body(Body::empty()).unwrap()).await;
        assert_eq!(page.status(), StatusCode::OK, "{environment:?}");
        assert_eq!(page.headers()[header::X_FRAME_OPTIONS], "DENY");
        assert_eq!(page.headers()[header::CACHE_CONTROL], "no-store");
        let csp = page.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .to_owned();
        let cookie = page.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .to_owned();
        // Plain HTTP outside staging and production: no HSTS, no `Secure`.
        assert!(
            !page
                .headers()
                .contains_key(header::STRICT_TRANSPORT_SECURITY)
        );
        assert!(cookie.starts_with("rullst_csrf="), "{cookie}");
        assert!(!cookie.contains("Secure"), "{cookie}");
        let body = axum::body::to_bytes(page.into_body(), 1024).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        let (nonce, csrf) = body.split_once('|').unwrap();
        assert!(csp.contains(&format!("'nonce-{nonce}'")), "{csp}");
        assert!(!csrf.is_empty());

        let refused = send(&app, Request::post("/write").body(Body::empty()).unwrap()).await;
        assert_eq!(refused.status(), StatusCode::FORBIDDEN, "{environment:?}");
        assert!(
            refused
                .headers()
                .contains_key(header::CONTENT_SECURITY_POLICY)
        );
        let accepted = Request::post("/write")
            .header(header::COOKIE, format!("rullst_csrf={TOKEN}"))
            .header("x-csrf-token", TOKEN)
            .body(Body::empty())
            .unwrap();
        assert_eq!(send(&app, accepted).await.status(), StatusCode::NO_CONTENT);
    }
}

#[tokio::test]
async fn hsts_is_sent_over_https_in_every_environment_and_always_in_production() {
    let hsts = |response: &axum::response::Response| {
        response
            .headers()
            .get(header::STRICT_TRANSPORT_SECURITY)
            .map(|value| value.to_str().unwrap().to_owned())
    };
    let expected = Some("max-age=31536000; includeSubDomains".to_owned());
    for environment in [Environment::Development, Environment::Test] {
        let app = app(environment);
        let https = send(
            &app,
            Request::get("https://app.example/")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(hsts(&https), expected, "{environment:?}");
        let cookie = https.headers()[header::SET_COOKIE].to_str().unwrap();
        assert!(cookie.ends_with("; Secure"), "{cookie}");
        let plain = send(
            &app,
            Request::get("http://app.example/")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(hsts(&plain), None, "{environment:?}");
    }
    for environment in [Environment::Staging, Environment::Production] {
        let plain = send(
            &app(environment),
            Request::get("/").body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(hsts(&plain), expected, "{environment:?}");
    }
}

#[tokio::test]
async fn a_policy_the_route_sets_is_kept_and_every_other_route_gets_the_default() {
    let own = "default-src 'none'; script-src 'self'";
    let app = Router::new()
        .route(
            "/own",
            get(move || async move {
                (
                    [(header::CONTENT_SECURITY_POLICY, own)],
                    axum::response::Html("own"),
                )
            }),
        )
        .route(
            "/default",
            get(|| async { axum::response::Html("default") }),
        );
    for environment in [Environment::Production, Environment::Development] {
        let app =
            apply_security_baseline(app.clone(), SecurityConfig::default(), environment).unwrap();
        let kept = send(&app, Request::get("/own").body(Body::empty()).unwrap()).await;
        assert_eq!(
            kept.headers()[header::CONTENT_SECURITY_POLICY],
            own,
            "{environment:?}"
        );
        assert_eq!(
            kept.headers()
                .get_all(header::CONTENT_SECURITY_POLICY)
                .iter()
                .count(),
            1
        );
        let default = send(&app, Request::get("/default").body(Body::empty()).unwrap()).await;
        let policy = default.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap();
        assert!(
            policy.starts_with("default-src 'self'; base-uri 'self';"),
            "{policy}"
        );
        assert!(policy.contains("script-src 'self' 'nonce-"), "{policy}");
    }
}

#[tokio::test]
async fn pii_masking_runs_only_where_enabled_in_staging_and_production() {
    const PAGE: &str = "contact ada.lovelace@example.com or 123.456.789-09";
    let masked = crate::security::mask_pii(PAGE);
    assert_ne!(masked, PAGE);
    let body = |environment: Environment, enabled: bool| async move {
        let security = SecurityConfig {
            enable_pii_masking: enabled,
            ..SecurityConfig::default()
        };
        let app = Router::new().route("/", get(|| async { PAGE }));
        let app = apply_security_baseline(app, security, environment).unwrap();
        let response = send(&app, Request::get("/").body(Body::empty()).unwrap()).await;
        let bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    };
    assert_eq!(body(Environment::Production, true).await, masked);
    assert_eq!(body(Environment::Production, false).await, PAGE);
    assert_eq!(body(Environment::Development, true).await, PAGE);
}
