#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::config::{Environment, SecurityConfig};
use crate::security::{TrustedProxyConfig, TrustedProxyLayer, csrf_middleware};
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::routing::get;
use axum::{Router, middleware};
use std::net::SocketAddr;
use tower::ServiceExt;

const SESSION: &str = "rullst_session=token; Path=/; HttpOnly; SameSite=Lax; Max-Age=60";

#[test]
fn only_framework_cookies_without_secure_need_it() {
    for (cookie, expected) in [
        (SESSION, true),
        (" rullst_csrf=abc; Path=/", true),
        ("rullst_x=1", true),
        ("rullst_x=1; SecureX", true),
        ("rullst_session=a; Path=/; Secure", false),
        ("rullst_session=a;secure", false),
        ("rullst_session=a; SECURE=1", false),
        ("theme=dark; Path=/", false),
        ("my_rullst_x=1", false),
    ] {
        assert_eq!(
            needs_secure(&HeaderValue::from_static(cookie)),
            expected,
            "{cookie}"
        );
    }
}

#[test]
fn secure_is_appended_to_framework_cookies_only_and_order_is_kept() {
    let mut headers = HeaderMap::new();
    for cookie in [SESSION, "theme=dark", "rullst_csrf=abc; Secure"] {
        headers.append(header::SET_COOKIE, HeaderValue::from_static(cookie));
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    add_secure_to_framework_cookies(&mut headers);
    let cookies = headers
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        cookies,
        vec![
            "rullst_session=token; Path=/; HttpOnly; SameSite=Lax; Max-Age=60; Secure",
            "theme=dark",
            "rullst_csrf=abc; Secure"
        ]
    );
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
}

/// A request from `peer`, optionally with `X-Forwarded-For` and
/// `X-Forwarded-Proto: https`.
fn request(uri: &str, peer: &str, forwarded: bool) -> Request {
    let mut builder = Request::get(uri);
    if forwarded {
        builder = builder
            .header("x-forwarded-for", "203.0.113.9")
            .header("x-forwarded-proto", "https");
    }
    let mut request = builder.body(Body::empty()).unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    request
}

fn proxied(app: Router, trust_forwarded_proto: bool) -> Router {
    app.layer(TrustedProxyLayer::new(
        TrustedProxyConfig::new(["10.0.0.0/8"])
            .unwrap()
            .trust_forwarded_proto(trust_forwarded_proto),
    ))
}

/// Every `Set-Cookie` value of the response to `request`.
async fn cookies(app: &Router, request: Request) -> Vec<String> {
    let response = app.clone().oneshot(request).await.unwrap();
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap().to_owned())
        .collect()
}

fn login_app(environment: Environment, trust_forwarded_proto: bool) -> Router {
    let router = Router::new().route(
        "/",
        get(|| async { ([(header::SET_COOKIE, SESSION)], "signed in") }),
    );
    let app =
        crate::security::apply_security_baseline(router, SecurityConfig::default(), environment)
            .unwrap();
    proxied(app, trust_forwarded_proto)
}

#[tokio::test]
async fn the_baseline_marks_session_cookies_secure_over_https_in_every_environment() {
    let secure = format!("{SESSION}; Secure");
    for environment in [Environment::Development, Environment::Test] {
        let app = login_app(environment, true);
        // A trusted proxy reporting HTTPS.
        assert_eq!(
            cookies(&app, request("/", "10.0.0.1:443", true)).await,
            vec![secure.clone()]
        );
        // An HTTPS request URI (HTTP/2 over TLS).
        assert_eq!(
            cookies(&app, request("https://app.example/", "192.0.2.1:5000", false)).await,
            vec![secure.clone()]
        );
        // Plain HTTP, and a forwarded scheme from an untrusted peer.
        for request in [
            request("/", "10.0.0.1:443", false),
            request("/", "192.0.2.1:5000", true),
            request("http://app.example/", "192.0.2.1:5000", false),
        ] {
            assert_eq!(cookies(&app, request).await, vec![SESSION.to_string()]);
        }
        // A trusted proxy whose scheme is not trusted.
        let untrusted_scheme = login_app(environment, false);
        assert_eq!(
            cookies(&untrusted_scheme, request("/", "10.0.0.1:443", true)).await,
            vec![SESSION.to_string()]
        );
    }
}

fn csrf_app(environment: Environment) -> Router {
    let router = Router::new()
        .route("/", get(|| async { "form" }))
        .layer(middleware::from_fn(csrf_middleware))
        .layer(axum::Extension(environment));
    proxied(router, true)
}

#[tokio::test]
async fn the_csrf_cookie_is_secure_over_https_or_in_staging_and_production() {
    let development = csrf_app(Environment::Development);
    let plain = cookies(&development, request("/", "10.0.0.1:443", false)).await;
    assert_eq!(plain.len(), 1);
    assert!(!plain[0].contains("Secure"), "{plain:?}");
    for request in [
        request("/", "10.0.0.1:443", true),
        request("https://app.example/", "192.0.2.1:5000", false),
    ] {
        let secure = cookies(&development, request).await;
        assert!(secure[0].ends_with("; SameSite=Lax; Secure"), "{secure:?}");
    }
    for environment in [Environment::Staging, Environment::Production] {
        let app = csrf_app(environment);
        let cookie = cookies(&app, request("/", "10.0.0.1:443", false)).await;
        assert!(cookie[0].ends_with("; Secure"), "{cookie:?}");
    }
}

#[test]
fn https_evidence_is_the_uri_scheme_or_a_trusted_forwarded_scheme() {
    let extensions = Extensions::new();
    let https: Uri = "https://app.example/".parse().unwrap();
    let http: Uri = "http://app.example/".parse().unwrap();
    let origin: Uri = "/".parse().unwrap();
    assert!(request_is_https(&https, &extensions));
    assert!(!request_is_https(&http, &extensions));
    assert!(!request_is_https(&origin, &extensions));
}
