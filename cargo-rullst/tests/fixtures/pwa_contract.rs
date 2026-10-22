//! The `make:pwa` contract, compiled inside a generated Blank application by
//! cargo-rullst's `pwa_scaffold_cli` test as `src/pwa_contract.rs`. The
//! router is wrapped in the baseline `Server` adds in production.

use rullst::config::{Environment, SecurityConfig};
use rullst::web::axum::body::{Body, to_bytes};
use rullst::web::axum::extract::ConnectInfo;
use rullst::web::axum::http::{HeaderName, Request, StatusCode, header};
use rullst::web::axum::response::Response;
use rullst::web::tower::ServiceExt;

async fn send(request: Request<Body>, environment: Environment) -> Response {
    let app = rullst::security::apply_security_baseline(
        super::router().into_axum(),
        SecurityConfig::default(),
        environment,
    )
    .expect("security baseline");
    let mut request = request;
    let peer = std::net::SocketAddr::from(([127, 0, 0, 1], 40_000));
    request.extensions_mut().insert(ConnectInfo(peer));
    app.oneshot(request).await.expect("router response")
}

async fn get(uri: &str) -> Response {
    let request = Request::get(uri).body(Body::empty()).expect("GET request");
    send(request, Environment::Production).await
}

fn value(response: &Response, name: HeaderName) -> String {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

async fn text(response: Response) -> String {
    let bytes = to_bytes(response.into_body(), 1 << 20).await.expect("body");
    String::from_utf8(bytes.to_vec()).expect("UTF-8 body")
}

#[tokio::test]
async fn the_worker_is_served_from_the_root_as_revalidated_javascript() {
    let response = get("/sw.js").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        value(&response, header::CONTENT_TYPE),
        "text/javascript; charset=utf-8"
    );
    assert_eq!(value(&response, header::CACHE_CONTROL), "no-cache");
    let source = text(response).await;
    assert!(source.contains(concat!(
        "const VERSION = '",
        env!("CARGO_PKG_VERSION"),
        "';"
    )));
    assert!(!source.contains("const VERSION = 'dev';"));
    assert!(source.contains("  '/offline',\n"));
}

#[tokio::test]
async fn the_offline_page_may_be_stored_but_pages_stay_no_store() {
    let offline = get("/offline").await;
    assert_eq!(offline.status(), StatusCode::OK);
    assert_eq!(value(&offline, header::CACHE_CONTROL), "no-cache");
    assert!(value(&offline, header::CONTENT_TYPE).starts_with("text/html"));
    assert!(text(offline).await.contains("You are offline"));

    let home = get("/").await;
    assert_eq!(home.status(), StatusCode::OK);
    // The worker never stores a no-store response, the baseline default.
    assert_eq!(value(&home, header::CACHE_CONTROL), "no-store");
    let policy = value(&home, header::CONTENT_SECURITY_POLICY);
    // /sw.js, the manifest and /static/pwa.js are same-origin; without a
    // manifest-src directive the manifest falls back to default-src 'self'.
    for directive in ["default-src 'self'", "script-src 'self'", "worker-src 'self'"] {
        assert!(policy.contains(directive), "{policy}");
    }
    assert!(!policy.contains("manifest-src"), "{policy}");
    let page = text(home).await;
    for tag in [
        r#"<link rel="manifest" href="/static/manifest.webmanifest" />"#,
        r##"<meta name="theme-color" content="#0f172a" />"##,
        r#"<link rel="apple-touch-icon" href="/static/icons/apple-touch-icon.png" />"#,
        r#"<script src="/static/pwa.js" defer="defer"></script></head>"#,
    ] {
        assert!(page.contains(tag), "{tag} missing from {page}");
    }
}

#[tokio::test]
async fn asset_links_follow_the_project_setting() {
    // pwa_scaffold_cli sets RULLST_ANDROID_ASSET_LINKS for this process.
    let response = get("/.well-known/assetlinks.json").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(value(&response, header::CONTENT_TYPE), "application/json");
    assert_eq!(
        value(&response, header::CACHE_CONTROL),
        "public, max-age=3600"
    );
    let statements: serde_json::Value =
        serde_json::from_str(&text(response).await).expect("JSON statement list");
    assert_eq!(
        statements[0]["relation"][0],
        "delegate_permission/common.handle_all_urls"
    );
    assert_eq!(statements[0]["target"]["package_name"], "dev.acme.notes");
}

#[tokio::test]
async fn only_get_is_answered() {
    // Development has no CSRF layer, so the method reaches the router.
    let request = Request::post("/sw.js").body(Body::empty()).expect("POST request");
    let response = send(request, Environment::Development).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
