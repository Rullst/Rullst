//! Validation failures reach the page under htmx's default swap rule.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    routing::post,
};
use rullst_core::{Validate, ValidatedForm};
use tower::ServiceExt;

#[derive(serde::Deserialize, Validate)]
struct Signup {
    #[validate(length(min = 3, message = "Username too short"))]
    username: String,
}

fn app() -> Router {
    Router::new().route(
        "/signup",
        post(|ValidatedForm(signup): ValidatedForm<Signup>| async move { signup.username }),
    )
}

/// The swap predicate of the pinned htmx 1.9.12 client
/// (`status>=200&&status<400&&status!==204`), also htmx 2's default.
fn htmx_swaps(status: StatusCode) -> bool {
    status.as_u16() >= 200 && status.as_u16() < 400 && status != StatusCode::NO_CONTENT
}

async fn submit(body: &'static str, content_type: &str, htmx: bool) -> axum::response::Response {
    let mut request = Request::post("/signup").header(header::CONTENT_TYPE, content_type);
    if htmx {
        request = request.header("HX-Request", "true");
    }
    app()
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn htmx_validation_fragments_are_swappable_and_labelled() {
    let response = submit("username=ab", "application/x-www-form-urlencoded", true).await;
    assert!(htmx_swaps(response.status()), "{}", response.status());
    assert_eq!(response.headers()["x-rullst-validation-status"], "422");
    let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("Validation Failed"), "{body}");
    assert!(body.contains("Username too short"), "{body}");

    let response = submit("username=abc", "text/plain", true).await;
    assert!(htmx_swaps(response.status()), "{}", response.status());
    assert_eq!(response.headers()["x-rullst-validation-status"], "415");
}

#[tokio::test]
async fn non_htmx_clients_keep_rest_status_codes() {
    let response = submit("username=ab", "application/x-www-form-urlencoded", false).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        !response
            .headers()
            .contains_key("x-rullst-validation-status")
    );

    let response = submit("username=abc", "text/plain", false).await;
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let response = submit("username=abc", "application/x-www-form-urlencoded", true).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        !response
            .headers()
            .contains_key("x-rullst-validation-status")
    );
}
