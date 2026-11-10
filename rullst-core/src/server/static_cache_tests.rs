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

#[tokio::test]
async fn the_working_directory_mount_keeps_the_application_routes_and_its_manifest() {
    let app = {
        let _guard = crate::assets::TEST_MANIFEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let other = static_root();
        crate::assets::activate(static_manifest(other.path()));
        assert_ne!(crate::assets::url("site.css"), "/static/site.css");

        let app = axum::Router::new().route("/page", axum::routing::get(|| async { "dynamic" }));
        let app = mount_static_assets(app);
        // The crate directory has no `static/`: the mount replaced the manifest.
        assert_eq!(crate::assets::url("site.css"), "/static/site.css");
        crate::assets::deactivate();
        app
    };
    let response = app
        .oneshot(Request::builder().uri("/page").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

/// `site.css` as `crate::assets::url` links it while `root` is mounted.
fn hashed_site_css(root: &std::path::Path) -> String {
    static_manifest(root).url("site.css")
}

async fn body(
    app: &axum::Router,
    path: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut request = Request::builder().uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    (status, headers, bytes.to_vec())
}

#[tokio::test]
async fn hashed_names_serve_the_real_file_immutable_with_its_validators() {
    let root = static_root();
    let app = application(root.path());
    let hashed = hashed_site_css(root.path());
    assert!(
        hashed.starts_with("/static/site.") && hashed.ends_with(".css"),
        "{hashed}"
    );

    let (status, headers, content) = body(&app, &hashed, &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], IMMUTABLE);
    assert_eq!(content, b"main{}");
    let etag = headers[header::ETAG].to_str().unwrap().to_string();
    let (status, headers, _) = body(&app, &hashed, &[("if-none-match", &etag)]).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert_eq!(headers[header::CACHE_CONTROL], IMMUTABLE);

    // The plain name keeps revalidating.
    let (status, headers, _) = body(&app, "/static/site.css", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], REVALIDATE);
}

#[tokio::test]
async fn hashed_names_keep_brotli_and_zstandard_siblings() {
    let root = static_root();
    std::fs::write(root.path().join("static/site.css.br"), "brotli body").unwrap();
    std::fs::write(root.path().join("static/site.css.zst"), "zstd body").unwrap();
    let app = application(root.path());
    let hashed = hashed_site_css(root.path());
    for (encoding, expected) in [("br", "brotli body"), ("zstd", "zstd body")] {
        let (status, headers, content) =
            body(&app, &hashed, &[("accept-encoding", encoding)]).await;
        assert_eq!(status, StatusCode::OK, "{encoding}");
        assert_eq!(headers[header::CONTENT_ENCODING], encoding);
        let content_type = headers[header::CONTENT_TYPE].to_str().unwrap();
        assert!(
            content_type.starts_with("text/css"),
            "{encoding}: {content_type}"
        );
        assert_eq!(headers[header::CACHE_CONTROL], IMMUTABLE, "{encoding}");
        assert_eq!(content, expected.as_bytes(), "{encoding}");
    }
}

#[tokio::test]
async fn an_outdated_hash_redirects_without_caching_and_unknown_ones_stay_404() {
    let root = static_root();
    let app = application(root.path());
    let old = hashed_site_css(root.path());
    std::fs::write(root.path().join("static/site.css"), "main{color:red}").unwrap();
    let current = hashed_site_css(root.path());
    assert_ne!(old, current);
    for method in ["GET", "HEAD"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(&old)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::TEMPORARY_REDIRECT,
            "{method}"
        );
        assert_eq!(response.headers()[header::LOCATION], current.as_str());
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    let (status, headers, content) = body(&app, &current, &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], IMMUTABLE);
    assert_eq!(content, b"main{color:red}");

    for unknown in [
        "/static/missing.0123456789abcdef.css",
        "/static/../static/site.0123456789abcdef.css",
        "/static/site%2e0123456789abcdef.css",
    ] {
        let (status, headers, _) = body(&app, unknown, &[]).await;
        assert!(status.is_client_error(), "{unknown}: {status}");
        assert_eq!(headers[header::CACHE_CONTROL], "no-store", "{unknown}");
    }
}

#[tokio::test]
async fn other_methods_and_real_hash_like_files_are_not_resolved() {
    let root = static_root();
    let app = application(root.path());
    let hashed = hashed_site_css(root.path());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&hashed)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(response.status(), StatusCode::OK);
    assert_ne!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        IMMUTABLE
    );

    // `app.3f9a2c1b.css` exists: it is served as it is.
    let (status, headers, content) = body(&app, &format!("/static/{HASHED}"), &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], IMMUTABLE);
    assert_eq!(content, b"body{}");
}
