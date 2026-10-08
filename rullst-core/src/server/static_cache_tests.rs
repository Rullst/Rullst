#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::config::{Environment, SecurityConfig};
use axum::body::Body;
use axum::http::{HeaderMap, Request};
use tower::ServiceExt;

const HASHED: &str = "app.3f9a2c1b.css";

#[test]
fn only_hex_content_hashes_mark_a_file_name_as_fingerprinted() {
    for name in [
        HASHED,
        "chunk-3f9a2c1b.js",
        "app.3f9a2c1b.js.map",
        "vendor.0123456789abcdef0123456789abcdef.js",
        "logo-v2-cafe1234.png",
    ] {
        assert!(is_fingerprinted(name), "{name}");
    }
    for name in [
        "app.css",
        "app-settings.js",
        "report-20261008.pdf",
        "app.deadbeefcafe.css",
        "app.3F9A2C1B.css",
        "app.3f9a2c1.css",
        "3f9a2c1b.css",
        ".3f9a2c1b.css",
        "app-3f9a2c1b",
        "jquery-3.7.1.min.js",
        "app.3f9a2c1bz.css",
    ] {
        assert!(!is_fingerprinted(name), "{name}");
    }
    assert_eq!(
        static_cache_policy("/static/js/chunk-3f9a2c1b.js"),
        Some(IMMUTABLE)
    );
    assert_eq!(static_cache_policy("/static/js/app.js"), Some(REVALIDATE));
    assert_eq!(static_cache_policy("/static/docs/"), Some(REVALIDATE));
    assert_eq!(static_cache_policy("/assets/app.3f9a2c1b.css"), None);
}

/// The production composition: a dynamic route, the static mount and the
/// security baseline around both.
fn application(root: &std::path::Path) -> axum::Router {
    let app = axum::Router::new().route("/page", axum::routing::get(|| async { "dynamic" }));
    let app = mount_static_assets_at(app, root);
    crate::security::apply_security_baseline(
        app,
        SecurityConfig::default(),
        Environment::Production,
    )
    .unwrap()
}

async fn get(app: &axum::Router, path: &str, headers: &[(&str, &str)]) -> (StatusCode, HeaderMap) {
    let mut request = Request::builder().uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    (response.status(), response.headers().clone())
}

fn static_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("static")).unwrap();
    std::fs::write(root.path().join("static").join(HASHED), "body{}").unwrap();
    std::fs::write(root.path().join("static/app.3f9a2c1b.css.zst"), "zstd").unwrap();
    std::fs::write(root.path().join("static/site.css"), "main{}").unwrap();
    root
}

#[tokio::test]
async fn fingerprinted_assets_are_immutable_in_every_encoding() {
    let root = static_root();
    let app = application(root.path());
    let path = format!("/static/{HASHED}");
    for encoding in ["identity", "zstd"] {
        let (status, headers) = get(&app, &path, &[("accept-encoding", encoding)]).await;
        assert_eq!(status, StatusCode::OK, "{encoding}");
        assert_eq!(headers[header::CACHE_CONTROL], IMMUTABLE, "{encoding}");
    }
}

#[tokio::test]
async fn plain_assets_revalidate_and_answer_matching_validators_with_304() {
    let root = static_root();
    let app = application(root.path());
    let (status, headers) = get(&app, "/static/site.css", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], REVALIDATE);
    let etag = headers[header::ETAG].to_str().unwrap().to_string();
    let modified = headers[header::LAST_MODIFIED].to_str().unwrap().to_string();

    for validator in [
        ("if-none-match", etag.as_str()),
        ("if-modified-since", &modified),
    ] {
        let (status, headers) = get(&app, "/static/site.css", &[validator]).await;
        assert_eq!(status, StatusCode::NOT_MODIFIED, "{}", validator.0);
        assert_eq!(
            headers[header::CACHE_CONTROL],
            REVALIDATE,
            "{}",
            validator.0
        );
    }
    let (status, _) = get(&app, "/static/site.css", &[("if-none-match", "\"other\"")]).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn dynamic_responses_and_static_errors_keep_no_store() {
    let root = static_root();
    let app = application(root.path());
    let (status, headers) = get(&app, "/page", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    let (status, headers) = get(&app, "/static/missing.3f9a2c1b.css", &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
}
