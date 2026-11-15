//! `GET` requests for static files and the CSRF token: the configured
//! `csrf_static_prefixes` and `csrf_static_paths`, not file extensions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::config::SecurityConfig;
use axum::{Extension, Router, body::Body, routing::get};
use tower::ServiceExt;

/// Answers every path with the request's `CsrfToken`, or `none`; `security`
/// is installed as the request's `SecurityConfig` when given.
fn token_echo_app(security: Option<SecurityConfig>) -> Router {
    let echo = |token: Option<Extension<CsrfToken>>| async move {
        token.map_or_else(|| "none".to_owned(), |Extension(token)| token.0)
    };
    let app = Router::new()
        .fallback(get(echo).post(|| async { "written" }))
        .layer(axum::middleware::from_fn(csrf_middleware));
    match security {
        Some(security) => app.layer(Extension(security)),
        None => app,
    }
}

/// The body and the number of `Set-Cookie` headers of `method path`.
async fn visit(
    app: &Router,
    method: axum::http::Method,
    path: &str,
    cookie: Option<&str>,
) -> (StatusCode, String, usize) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .count();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap(), cookies)
}

async fn get_page(app: &Router, path: &str, cookie: Option<&str>) -> (String, usize) {
    let (status, body, cookies) = visit(app, axum::http::Method::GET, path, cookie).await;
    assert_eq!(status, StatusCode::OK, "{path}");
    (body, cookies)
}

/// Whether a first `GET` of `path` mints a token and sets it: the page gets
/// the 32-character token masked, 86 characters.
async fn first_visit_mints(app: &Router, path: &str) -> bool {
    match get_page(app, path, None).await {
        (token, 1) if token.len() == 86 => true,
        (token, 0) if token == "none" => false,
        other => panic!("{path}: unexpected {other:?}"),
    }
}

#[test]
fn the_default_static_files_are_the_generated_projects_ones() {
    let security = SecurityConfig::default();
    assert_eq!(security.csrf_static_prefixes, ["/static/", "/.well-known/"]);
    assert_eq!(
        security.csrf_static_paths,
        [
            "/favicon.ico",
            "/robots.txt",
            "/sitemap.xml",
            "/manifest.webmanifest",
            "/sw.js"
        ]
    );
    for path in [
        "/static/app.css",
        "/static/fonts/inter.woff2",
        "/static/report",
        "/.well-known/assetlinks.json",
        "/favicon.ico",
        "/robots.txt",
        "/sitemap.xml",
        "/manifest.webmanifest",
        "/sw.js",
    ] {
        assert!(security.is_csrf_static_path(path), "{path}");
    }
    for path in [
        "/report.json",
        "/feed.xml",
        "/notes.txt",
        "/app.js",
        "/logo.png",
        "/statics/report",
        "/static",
        "/robots.txt/x",
        "/x/robots.txt",
        "/dashboard",
        "/",
    ] {
        assert!(!security.is_csrf_static_path(path), "{path}");
    }
}

#[test]
fn builder_methods_add_to_the_defaults() {
    let security = SecurityConfig::default()
        .with_csrf_static_prefix("/assets/")
        .with_csrf_static_path("/humans.txt");
    assert_eq!(
        security.csrf_static_prefixes,
        ["/static/", "/.well-known/", "/assets/"]
    );
    assert_eq!(security.csrf_static_paths.last().unwrap(), "/humans.txt");
    assert_eq!(security.csrf_static_paths.len(), 6);
    assert!(security.is_csrf_static_path("/assets/app.js"));
    assert!(security.is_csrf_static_path("/humans.txt"));
    assert!(security.is_csrf_static_path("/static/app.js"));
    assert!(!security.is_csrf_static_path("/humans.txt.bak"));

    // Only prefixes, or only paths: each list decides on its own.
    let mut prefixes_only = SecurityConfig::default();
    prefixes_only.csrf_static_paths.clear();
    assert!(prefixes_only.is_csrf_static_path("/static/app.js"));
    assert!(!prefixes_only.is_csrf_static_path("/robots.txt"));
    let mut paths_only = SecurityConfig::default();
    paths_only.csrf_static_prefixes.clear();
    assert!(paths_only.is_csrf_static_path("/robots.txt"));
    assert!(!paths_only.is_csrf_static_path("/static/app.js"));
}

#[tokio::test]
async fn dynamic_pages_get_a_token_whatever_their_extension() {
    let app = token_echo_app(None);
    for path in ["/report.json", "/feed.xml", "/notes.txt", "/form"] {
        assert!(first_visit_mints(&app, path).await, "{path}");
    }
    for path in [
        "/static/app.css",
        "/.well-known/assetlinks.json",
        "/robots.txt",
        "/sw.js",
    ] {
        assert!(!first_visit_mints(&app, path).await, "{path}");
    }
}

#[tokio::test]
async fn static_files_reuse_an_existing_cookie_token_without_setting_one() {
    let app = token_echo_app(None);
    for path in ["/static/page", "/favicon.ico"] {
        let (token, cookies) =
            get_page(&app, path, Some("theme=dark; rullst_csrf=existingToken123")).await;
        assert_eq!(cookies, 0, "{path}");
        assert!(
            bool::from(super::super::csrf_mask::csrf_proof_matches(
                token.as_bytes(),
                "existingToken123"
            )),
            "{path}: {token}"
        );
        // A malformed cookie is not a token.
        assert_eq!(
            get_page(&app, path, Some("rullst_csrf=not valid")).await,
            ("none".to_owned(), 0),
            "{path}"
        );
    }
    // HEAD mirrors GET: no cookie on a static file, one on a page.
    let (_, _, cookies) = visit(&app, axum::http::Method::HEAD, "/static/app.css", None).await;
    assert_eq!(cookies, 0);
    let (_, _, cookies) = visit(&app, axum::http::Method::HEAD, "/report.json", None).await;
    assert_eq!(cookies, 1);
}

#[tokio::test]
async fn the_request_security_config_chooses_the_static_files() {
    let app = token_echo_app(Some(
        SecurityConfig::default()
            .with_csrf_static_prefix("/assets/")
            .with_csrf_static_path("/humans.txt"),
    ));
    for path in ["/assets/app.js", "/humans.txt", "/static/app.css"] {
        assert!(!first_visit_mints(&app, path).await, "{path}");
    }
    assert!(first_visit_mints(&app, "/report.json").await);

    // A replaced list drops the defaults it does not repeat.
    let security = SecurityConfig {
        csrf_static_prefixes: vec!["/assets/".to_owned()],
        csrf_static_paths: Vec::new(),
        ..SecurityConfig::default()
    };
    let app = token_echo_app(Some(security));
    assert!(!first_visit_mints(&app, "/assets/app.js").await);
    assert!(first_visit_mints(&app, "/static/page").await);
    assert!(first_visit_mints(&app, "/robots.txt").await);
}

#[tokio::test]
async fn static_files_do_not_relax_state_changing_requests() {
    let app = token_echo_app(None);
    for path in ["/static/upload", "/robots.txt", "/report.json"] {
        let (status, _, _) = visit(&app, axum::http::Method::POST, path, None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
        let (status, _, _) = visit(
            &app,
            axum::http::Method::POST,
            path,
            Some("rullst_csrf=existingToken123"),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
    }
}
