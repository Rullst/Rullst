//! A CSRF layer composed without `apply_security_baseline` has no
//! per-application `SecurityConfig`, so it must honour the process-global
//! configuration. This binary is the only one that sets that global, and it
//! does so before any request runs.
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::get,
};
use rullst_core::{config::RullstConfig, security::csrf_middleware};
use tower::ServiceExt;

#[tokio::test]
async fn a_standalone_csrf_layer_uses_the_global_same_site_policy() {
    let mut config = RullstConfig::default();
    config.security.csrf_same_site = "Strict".to_string();
    assert!(RullstConfig::set_global(config).is_ok());

    let app = Router::new()
        .route("/form", get(|| async { "form" }))
        .layer(axum::middleware::from_fn(csrf_middleware));
    let response = app
        .oneshot(Request::get("/form").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let set_cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(set_cookie.starts_with("rullst_csrf="));
    assert!(set_cookie.contains("; SameSite=Strict"));
}
