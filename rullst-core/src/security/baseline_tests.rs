#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::field_reassign_with_default
)]

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
