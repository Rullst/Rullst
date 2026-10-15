//! `GET` requests for static-looking paths and the CSRF token.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::{Extension, Router, body::Body, routing::get};
use tower::ServiceExt;

/// Answers with the request's `CsrfToken`, or `none`.
fn token_echo_app() -> Router {
    let echo = |token: Option<Extension<CsrfToken>>| async move {
        token.map_or_else(|| "none".to_owned(), |Extension(token)| token.0)
    };
    Router::new()
        .route("/reports/summary.json", get(echo))
        .route("/static/page", get(echo))
        .route("/form", get(echo))
        .layer(axum::middleware::from_fn(csrf_middleware))
}

async fn get_page(path: &str, cookie: Option<&str>) -> (String, usize) {
    let mut request = Request::get(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let response = token_echo_app()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "{path}");
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .count();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    (String::from_utf8(body.to_vec()).unwrap(), cookies)
}

#[tokio::test]
async fn static_looking_paths_reuse_an_existing_cookie_token_without_setting_one() {
    for path in ["/reports/summary.json", "/static/page"] {
        assert_eq!(
            get_page(path, Some("theme=dark; rullst_csrf=existingToken123")).await,
            ("existingToken123".to_owned(), 0),
            "{path}"
        );
        // A first visit gets neither a token nor a cookie there.
        assert_eq!(get_page(path, None).await, ("none".to_owned(), 0), "{path}");
        // A malformed cookie is not a token.
        assert_eq!(
            get_page(path, Some("rullst_csrf=not valid")).await,
            ("none".to_owned(), 0),
            "{path}"
        );
    }
    // Other pages still mint a token and set it as the cookie.
    let (token, cookies) = get_page("/form", None).await;
    assert_eq!((token.len(), cookies), (32, 1));
}
