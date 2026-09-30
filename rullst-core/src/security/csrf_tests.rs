//! Double-submit cookie middleware tests.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]

use super::*;
use axum::{Router, body::Body, http::Request, routing::any};
use tower::ServiceExt;

#[tokio::test]
async fn empty_or_ambiguous_csrf_proofs_do_not_reach_the_handler() {
    let token = generate_csrf_token();
    let cookie = format!("rullst_csrf={token}");
    let app = Router::new()
        .route("/write", any(|| async { StatusCode::NO_CONTENT }))
        .layer(axum::middleware::from_fn(csrf_middleware));
    let requests = [
        Request::post("/write")
            .header(header::COOKIE, "rullst_csrf=")
            .header("x-csrf-token", "")
            .body(Body::empty())
            .unwrap(),
        Request::post("/write")
            .header(header::COOKIE, "rullst_csrf=")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from("_token="))
            .unwrap(),
        Request::post("/write")
            .header(header::COOKIE, format!("{cookie}; {cookie}"))
            .header("x-csrf-token", &token)
            .body(Body::empty())
            .unwrap(),
        Request::post("/write")
            .header(header::COOKIE, &cookie)
            .header(header::COOKIE, &cookie)
            .header("x-csrf-token", &token)
            .body(Body::empty())
            .unwrap(),
        Request::post("/write")
            .header(header::COOKIE, &cookie)
            .header("x-csrf-token", &token)
            .header("x-csrf-token", "different-token")
            .body(Body::empty())
            .unwrap(),
        Request::post("/write")
            .header(header::COOKIE, &cookie)
            .header(
                header::CONTENT_TYPE,
                "text/plain; application/x-www-form-urlencoded",
            )
            .body(Body::from(format!("_token={token}")))
            .unwrap(),
        Request::post("/write")
            .header(header::COOKIE, format!("rullst_csrf={}", "x".repeat(129)))
            .header("x-csrf-token", "x".repeat(129))
            .body(Body::empty())
            .unwrap(),
    ];
    for request in requests {
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }
}

#[tokio::test]
async fn split_cookie_fields_support_one_unambiguous_valid_csrf_token() {
    let token = generate_csrf_token();
    let app = Router::new()
        .route("/write", any(|| async { StatusCode::NO_CONTENT }))
        .layer(axum::middleware::from_fn(csrf_middleware));
    let request = Request::post("/write")
        .header(header::COOKIE, "other_cookie=value")
        .header(header::COOKIE, format!("rullst_csrf={token}"))
        .header("x-csrf-token", token)
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.oneshot(request).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn safe_http_methods_do_not_require_a_token() {
    let app = Router::new()
        .route("/", any(|| async { StatusCode::OK }))
        .layer(axum::middleware::from_fn(csrf_middleware));

    for method in [
        axum::http::Method::HEAD,
        axum::http::Method::OPTIONS,
        axum::http::Method::TRACE,
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(response.status(), StatusCode::FORBIDDEN);
    }
}

#[tokio::test]
async fn production_like_environment_sets_secure_cookie() {
    let app = Router::new()
        .route("/", any(|| async { StatusCode::OK }))
        .layer(axum::middleware::from_fn(csrf_middleware))
        .layer(axum::Extension(crate::config::Environment::Staging));

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cookie.contains("; Secure"));
}

#[tokio::test]
async fn nested_csrf_layers_emit_one_matching_cookie_and_accept_the_post() {
    use axum::{Extension, routing::get};

    let app =
        Router::new()
            .route(
                "/form",
                get(|Extension(token): Extension<CsrfToken>| async move {
                    token.as_str().to_owned()
                })
                .post(
                    |Extension(token): Extension<CsrfToken>| async move {
                        token.as_str().to_owned()
                    },
                ),
            )
            .layer(axum::middleware::from_fn(csrf_middleware))
            .layer(axum::middleware::from_fn(csrf_middleware));

    let response = app
        .clone()
        .oneshot(Request::get("/form").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let cookies = response.headers().get_all(header::SET_COOKIE);
    assert_eq!(cookies.iter().count(), 1);
    let cookie = cookies.iter().next().unwrap().to_str().unwrap().to_owned();
    let body = axum::body::to_bytes(response.into_body(), 128)
        .await
        .unwrap();
    let token = std::str::from_utf8(&body).unwrap();
    assert!(cookie.starts_with(&format!("rullst_csrf={token};")));

    let posted = app
        .oneshot(
            Request::post("/form")
                .header(header::COOKIE, format!("rullst_csrf={token}"))
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("_token={token}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(posted.status(), StatusCode::OK);
    let posted_body = axum::body::to_bytes(posted.into_body(), 128).await.unwrap();
    assert_eq!(posted_body.as_ref(), token.as_bytes());
}

#[tokio::test]
async fn only_exact_configured_post_webhook_path_is_exempt() {
    let mut security = crate::config::SecurityConfig::default();
    security.csrf_signed_webhook_paths = vec!["/billing/webhook".to_owned()];
    let app = Router::new()
        .route("/billing/webhook", any(|| async { StatusCode::OK }))
        .route("/billing/webhook/extra", any(|| async { StatusCode::OK }))
        .layer(axum::middleware::from_fn(csrf_middleware))
        .layer(axum::Extension(security));

    let exempt = app
        .clone()
        .oneshot(
            Request::builder()
                .method(axum::http::Method::POST)
                .uri("/billing/webhook")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(exempt.status(), StatusCode::OK);

    let prefix = app
        .clone()
        .oneshot(
            Request::builder()
                .method(axum::http::Method::POST)
                .uri("/billing/webhook/extra")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(prefix.status(), StatusCode::FORBIDDEN);

    let non_post = app
        .oneshot(
            Request::builder()
                .method(axum::http::Method::PUT)
                .uri("/billing/webhook")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(non_post.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn non_ascii_unrelated_cookies_do_not_hide_the_csrf_cookie() {
    let token = generate_csrf_token();
    let app = Router::new()
        .route("/write", any(|| async { StatusCode::NO_CONTENT }))
        .layer(axum::middleware::from_fn(csrf_middleware));
    let accented = header::HeaderValue::from_bytes("cidade=São Paulo".as_bytes()).unwrap();
    let combined = header::HeaderValue::from_bytes(
        format!("cidade=São Paulo; rullst_csrf={token}").as_bytes(),
    )
    .unwrap();

    let same_header = Request::post("/write")
        .header(header::COOKIE, combined.clone())
        .header("x-csrf-token", &token)
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(same_header).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );

    let split_headers = Request::post("/write")
        .header(header::COOKIE, accented)
        .header(header::COOKIE, format!("rullst_csrf={token}"))
        .header("x-csrf-token", &token)
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(split_headers).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );

    // A safe request keeps the existing token instead of rotating it.
    let safe = Request::get("/write")
        .header(header::COOKIE, combined)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(safe).await.unwrap();
    assert!(response.headers().get(header::SET_COOKIE).is_none());

    // The CSRF cookie itself must still be a bounded ASCII token.
    let non_ascii_token = Request::post("/write")
        .header(
            header::COOKIE,
            header::HeaderValue::from_bytes("rullst_csrf=tökén".as_bytes()).unwrap(),
        )
        .header("x-csrf-token", "tökén")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.oneshot(non_ascii_token).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}
