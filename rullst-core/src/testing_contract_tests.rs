use super::*;
use axum::{
    Json,
    extract::Request,
    http::{HeaderMap, HeaderValue, header},
    routing::{any, get, post},
};
use serde_json::{Value, json};

#[tokio::test]
async fn fluent_test_client_exercises_every_method_and_response_accessor() {
    let router = Router::new()
        .route(
            "/method",
            any(|request: Request| async move { request.method().to_string() }),
        )
        .route(
            "/json",
            post(|Json(value): Json<Value>| async move { Json(value) }),
        )
        .route(
            "/response",
            get(|| async {
                let mut headers = HeaderMap::new();
                headers.insert("x-rullst-test", HeaderValue::from_static("ready"));
                headers.append(
                    header::SET_COOKIE,
                    HeaderValue::from_static("theme=dark; HttpOnly"),
                );
                headers.append(
                    header::SET_COOKIE,
                    HeaderValue::from_static("session=bounded; Secure"),
                );
                (headers, Json(json!({"status": "ok"})))
            }),
        );
    let app = TestApp::new_with_limit(router, 4_096);

    for response in [
        app.get("/method").await,
        app.post("/method").await,
        app.put("/method").await,
        app.patch("/method").await,
        app.delete("/method").await,
    ] {
        response.assert_status(200).assert_dont_see("TRACE");
    }

    app.post("/json")
        .header("x-fixture", "accepted")
        .json(&json!({"course": "Rust"}))
        .await
        .assert_json(&json!({"course": "Rust"}));

    let response = app.get("/response").max_body_bytes(1_024).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-rullst-test"),
        Some(&HeaderValue::from_static("ready"))
    );
    response
        .assert_header("x-rullst-test", "ready")
        .assert_see("status")
        .assert_cookie("theme", "dark")
        .assert_has_cookie("session");
    assert_eq!(response.cookie_value("missing"), None);
}

#[tokio::test]
async fn raw_and_form_request_bodies_are_sent_without_implicit_mutation() {
    let router = Router::new()
        .route(
            "/raw",
            any(|body: String| async move { body }),
        )
        .route(
            "/form",
            post(|axum::Form(values): axum::Form<std::collections::HashMap<String, String>>| async move {
                values.get("lesson").cloned().unwrap_or_default()
            }),
        );
    let app = TestApp::new(router);
    app.patch("/raw")
        .body("exact body")
        .send()
        .await
        .assert_see("exact body");
    app.post("/form")
        .form(&[("lesson", "ownership")])
        .await
        .assert_see("ownership");
}
