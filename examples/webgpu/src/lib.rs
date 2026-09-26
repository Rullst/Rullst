//! A public, read-only demonstration. No GPU, database or provider runs on the server.
use rullst_core::{
    config::{Environment, SecurityConfig},
    security::{SecurityBaselineError, apply_security_baseline},
    server::{IntoResponse, Redirect, Response, get, header},
    web::axum::Router,
};

fn asset(content_type: &'static str, body: &'static str) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store"),
        ],
        body,
    )
        .into_response()
}

/// Exact embedded assets, with the production CSRF/WAF/header baseline even locally.
/// Mount application authentication separately if the lesson itself is private.
pub fn router() -> Result<Router, SecurityBaselineError> {
    let app = Router::new()
        .route("/", get(|| async { Redirect::temporary("/webgpu/") }))
        .route(
            "/webgpu/",
            get(|| async { asset("text/html; charset=utf-8", include_str!("../index.html")) }),
        )
        .route(
            "/webgpu/style.css",
            get(|| async { asset("text/css; charset=utf-8", include_str!("../style.css")) }),
        );
    let app = [
        ("/webgpu/app.mjs", include_str!("../app.mjs")),
        ("/webgpu/controller.mjs", include_str!("../controller.mjs")),
        ("/webgpu/waves.mjs", include_str!("../waves.mjs")),
        ("/webgpu/gpu.mjs", include_str!("../gpu.mjs")),
    ]
    .into_iter()
    .fold(app, |app, (path, source)| {
        app.route(
            path,
            get(move || async move { asset("text/javascript; charset=utf-8", source) }),
        )
    });
    apply_security_baseline(app, SecurityConfig::default(), Environment::Production)
}
