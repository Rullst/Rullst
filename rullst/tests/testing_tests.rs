use axum::{
    Json, Router,
    extract::Form,
    http::{HeaderValue, header},
    response::IntoResponse,
    routing::{get, post},
};
use rullst::testing::TestApp;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct StatusJson {
    status: String,
    count: i32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct EchoMessage {
    message: String,
}

#[derive(Serialize, Deserialize)]
struct FormPayload {
    name: String,
}

// Route handlers
async fn handle_root() -> &'static str {
    "Welcome to Rullst!"
}

async fn handle_json() -> impl IntoResponse {
    Json(StatusJson {
        status: "ok".to_string(),
        count: 42,
    })
}

async fn handle_echo(Json(payload): Json<EchoMessage>) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_static("session_id=12345; Path=/; HttpOnly"),
    );
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_static("theme=dark; Path=/"),
    );
    (headers, Json(payload))
}

async fn handle_form(Form(payload): Form<FormPayload>) -> String {
    format!("Hello, {}!", payload.name)
}

fn build_router() -> Router {
    Router::new()
        .route("/", get(handle_root))
        .route("/json", get(handle_json))
        .route("/echo", post(handle_echo))
        .route("/form", post(handle_form))
}

#[tokio::test]
async fn test_e2e_get_plain_text() {
    let app = TestApp::new(build_router());

    app.get("/")
        .await
        .assert_status(200)
        .assert_see("Welcome to Rullst")
        .assert_dont_see("Goodbye");
}

#[tokio::test]
async fn test_e2e_get_json() {
    let app = TestApp::new(build_router());

    let expected = StatusJson {
        status: "ok".to_string(),
        count: 42,
    };

    app.get("/json")
        .await
        .assert_status(200)
        .assert_json(&expected);
}

#[tokio::test]
async fn test_e2e_post_json_and_cookies() {
    let app = TestApp::new(build_router());

    let payload = EchoMessage {
        message: "Hello Rust!".to_string(),
    };

    let response = app.post("/echo").json(&payload).await;

    response
        .assert_status(200)
        .assert_json(&payload)
        .assert_has_cookie("session_id")
        .assert_cookie("session_id", "12345")
        .assert_cookie("theme", "dark")
        .assert_header("content-type", "application/json");

    assert_eq!(
        response.cookie_value("session_id"),
        Some("12345".to_string())
    );
    assert_eq!(response.cookie_value("nonexistent_cookie"), None);
}

#[tokio::test]
async fn test_e2e_post_form() {
    let app = TestApp::new(build_router());

    let payload = FormPayload {
        name: "Antigravity".to_string(),
    };

    app.post("/form")
        .form(&payload)
        .await
        .assert_status(200)
        .assert_see("Hello, Antigravity!");
}
