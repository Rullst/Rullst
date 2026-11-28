//! The baseline's own `Vary: HX-Request` step: when the server uses it, and
//! that it answers as the separate layer on the routes beside the `/static`
//! mount and a CORS policy.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn the_baseline_varies_only_when_nothing_sits_between_it_and_the_routes() {
    assert!(InnerLayers::default().vary_in_baseline());
    let one = |set: fn(&mut InnerLayers)| {
        let mut layers = InnerLayers::default();
        set(&mut layers);
        layers
    };
    for layers in [
        one(|layers| layers.access_log = true),
        one(|layers| layers.console = true),
        one(|layers| layers.pii_masking = true),
    ] {
        assert!(!layers.vary_in_baseline(), "{layers:?}");
    }
}

/// `application()` under the baseline in production, beside the `/static`
/// mount of `root`, with the `Vary` step in the baseline or in a separate
/// layer on the routes.
fn composed(
    root: &std::path::Path,
    security: SecurityConfig,
    vary_in_baseline: bool,
) -> axum::Router {
    let mut app = application().into_axum();
    if !vary_in_baseline {
        app = app.layer(axum::middleware::from_fn(crate::htmx::vary_on_htmx_request));
    }
    let app = crate::server::static_cache::mount_static_assets_at(app, root);
    crate::security::apply_server_baseline(app, security, Environment::Production, vary_in_baseline)
        .unwrap()
}

/// The `/static` mount beside a baseline that runs the `Vary` step answers
/// as beside the separate layer, which never sees the mount.
#[tokio::test]
async fn static_files_beside_a_varying_baseline_keep_their_headers() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("static")).unwrap();
    std::fs::write(root.path().join("static/page.html"), "<p>static</p>").unwrap();
    let requests = || {
        let get = |uri: &str| Request::get(uri).body(Body::empty()).unwrap();
        vec![
            ("static page", get("/static/page.html")),
            ("missing static page", get("/static/missing.html")),
            ("page", get("/page")),
        ]
    };
    let security = SecurityConfig::default;
    let separate = transcript_of(&composed(root.path(), security(), false), requests()).await;
    let fused = transcript_of(&composed(root.path(), security(), true), requests()).await;
    assert_eq!(fused, separate);
    let (static_page, page) = separate
        .split_once("page: 200\n  content-type: text/html; charset=utf-8\n  vary")
        .unwrap();
    assert!(
        static_page.starts_with("static page: 200\n  content-type: text/html"),
        "{separate}"
    );
    // `ServeDir` adds its own `Vary: accept-encoding`.
    assert!(!static_page.contains("HX-Request"), "{separate}");
    assert!(page.starts_with(": HX-Request"), "{separate}");
}

/// With a CORS policy the baseline runs in two middlewares around the CORS
/// layer, which adds its own `Vary` values after the step's.
#[tokio::test]
async fn a_cors_policy_keeps_the_order_of_the_vary_values() {
    let root = tempfile::tempdir().unwrap();
    let security = || SecurityConfig {
        cors_allow_origins: vec!["https://academy.example".to_string()],
        cors_allow_credentials: true,
        ..SecurityConfig::default()
    };
    let requests = || {
        let from = |method: &str, uri: &str| {
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::ORIGIN, "https://academy.example")
        };
        vec![
            ("page", from("GET", "/page").body(Body::empty()).unwrap()),
            (
                "own vary",
                from("GET", "/own-vary").body(Body::empty()).unwrap(),
            ),
            (
                "preflight",
                from("OPTIONS", "/write")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .body(Body::empty())
                    .unwrap(),
            ),
            (
                "write without token",
                from("POST", "/write").body(Body::empty()).unwrap(),
            ),
        ]
    };
    let separate = transcript_of(&composed(root.path(), security(), false), requests()).await;
    let fused = transcript_of(&composed(root.path(), security(), true), requests()).await;
    assert_eq!(fused, separate);
    assert!(
        separate.contains("  vary: HX-Request\n  vary: origin\n"),
        "{separate}"
    );
}
