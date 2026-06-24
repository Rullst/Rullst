use axum::{Router, routing::get};
use rullst::{error_console::catch_panic_middleware, testing::TestApp};

// A simple handler that explicitly panics
async fn panic_handler() {
    panic!("Opa! Algo deu errado no Rullst!");
}

fn build_panic_router() -> Router {
    Router::new()
        .route("/panic", get(panic_handler))
        .layer(axum::middleware::from_fn(catch_panic_middleware))
}

use tokio::sync::Mutex;
static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_error_console_catches_panic_and_renders_html() {
    let _guard = ENV_LOCK.lock().await;
    // Enable backtraces so frame capture works across all environments
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    let app = TestApp::new(build_panic_router());

    // 1. Send request that panics
    let response = app.get("/panic").await;

    // 2. Verify status code is 500 (Internal Server Error)
    response.assert_status(500);

    // 3. Verify that the response is the Self-Healing Console HTML
    response
        .assert_header("content-type", "text/html; charset=utf-8")
        .assert_see("Rullst Self-Healing Console")
        .assert_see("Opa! Algo deu errado no Rullst!")
        .assert_see("Source Code Snippet");

    // 4. Verify that the backtrace section is present
    // (The exact filename may vary by environment/platform)
    response.assert_see("Stack Trace");
}

#[tokio::test]
async fn test_error_console_find_source_location() {
    let bt = "   0: std::backtrace::Backtrace::create\n   1: my_app::main\n             at /src/main.rs:10";
    let loc = rullst::error_console::find_source_location(bt);
    assert!(loc.is_some());
    let (file, line) = loc.unwrap();
    assert_eq!(file, "/src/main.rs");
    assert_eq!(line, 10);
}

#[tokio::test]
async fn test_error_console_extract_source_context() {
    // Should return None for invalid paths (e.g. traversal)
    let ctx = rullst::error_console::extract_source_context("../../../etc/passwd", 1, 5);
    assert!(ctx.is_none());
}

use axum::extract::{ConnectInfo, Json, Query};
use rullst::error_console::{AutoFixPayload, ExplainQuery, handle_autofix, handle_explain};
use std::net::SocketAddr;

#[tokio::test]
async fn test_handle_explain_non_loopback() {
    let addr = SocketAddr::from(([192, 168, 1, 1], 12345));
    let query: ExplainQuery = serde_json::from_value(serde_json::json!({
        "file": "src/lib.rs",
        "line": 10,
        "err": "some error"
    }))
    .unwrap();

    let res = handle_explain(ConnectInfo(addr), Query(query)).await;
    let res_body = axum::response::IntoResponse::into_response(res);
    let bytes = axum::body::to_bytes(res_body.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(body_str.contains("Access denied"));
}

#[tokio::test]
async fn test_handle_explain_loopback_invalid_file() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    let query: ExplainQuery = serde_json::from_value(serde_json::json!({
        "file": "../../../etc/passwd",
        "line": 10,
        "err": "some error"
    }))
    .unwrap();

    let res = handle_explain(ConnectInfo(addr), Query(query)).await;
    let res_body = axum::response::IntoResponse::into_response(res);
    let bytes = axum::body::to_bytes(res_body.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(body_str.contains("Access denied"));
}

#[tokio::test]
async fn test_handle_autofix_non_loopback() {
    let addr = SocketAddr::from(([192, 168, 1, 1], 12345));
    let payload: AutoFixPayload = serde_json::from_value(serde_json::json!({
        "file_path": "src/lib.rs",
        "line": 10,
        "error_message": "some error"
    }))
    .unwrap();

    let res = handle_autofix(ConnectInfo(addr), Json(payload)).await;
    let res_body = axum::response::IntoResponse::into_response(res);
    let bytes = axum::body::to_bytes(res_body.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(body_str.contains("Access denied"));
}

#[tokio::test]
async fn test_handle_autofix_path_traversal() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    let payload: AutoFixPayload = serde_json::from_value(serde_json::json!({
        "file_path": "../../../etc/passwd",
        "line": 10,
        "error_message": "some error"
    }))
    .unwrap();

    let res = handle_autofix(ConnectInfo(addr), Json(payload)).await;
    let res_body = axum::response::IntoResponse::into_response(res);
    let bytes = axum::body::to_bytes(res_body.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(body_str.contains("Access denied"));
}

#[tokio::test]
async fn test_error_console_extract_source_context_valid() {
    let _ = std::fs::write("test_extract.rs", "line1\nline2\nline3\nline4\nline5");
    let ctx = rullst::error_console::extract_source_context("test_extract.rs", 3, 1);
    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.len(), 3);
    assert_eq!(ctx[0].0, 2);
    assert_eq!(ctx[0].1, "line2");
    assert_eq!(ctx[1].0, 3);
    assert_eq!(ctx[1].1, "line3");
    assert_eq!(ctx[1].2, true);

    let _ = std::fs::remove_file("test_extract.rs");
}

#[tokio::test]
async fn test_handle_explain_sensitive_file() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    let query: ExplainQuery = serde_json::from_value(serde_json::json!({
        "file": "Cargo.toml",
        "line": 10,
        "err": "some error"
    }))
    .unwrap();

    let res = handle_explain(ConnectInfo(addr), Query(query)).await;
    let res_body = axum::response::IntoResponse::into_response(res);
    let bytes = axum::body::to_bytes(res_body.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(body_str.contains("Access denied"));
}

#[tokio::test]
async fn test_handle_explain_wrong_extension() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    let _ = std::fs::write("test_explain.txt", "SECRET=1");
    let query: ExplainQuery = serde_json::from_value(serde_json::json!({
        "file": "test_explain.txt",
        "line": 10,
        "err": "some error"
    }))
    .unwrap();

    let res = handle_explain(ConnectInfo(addr), Query(query)).await;
    let res_body = axum::response::IntoResponse::into_response(res);
    let bytes = axum::body::to_bytes(res_body.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(body_str.contains("Access denied"));
    let _ = std::fs::remove_file("test_explain.txt");
}

#[tokio::test]
async fn test_handle_explain_valid_file_ai_offline() {
    let _guard = ENV_LOCK.lock().await;
    unsafe {
        std::env::set_var("RULLST_AI_PROVIDER", "invalid_provider");
    }
    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    let _ = std::fs::write("test_ai.rs", "fn main() {}");
    let query: ExplainQuery = serde_json::from_value(serde_json::json!({
        "file": "test_ai.rs",
        "line": 1,
        "err": "some error"
    }))
    .unwrap();

    let res = handle_explain(ConnectInfo(addr), Query(query)).await;
    let res_body = axum::response::IntoResponse::into_response(res);
    let bytes = axum::body::to_bytes(res_body.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(
        body_str.contains("AI Engine offline") || body_str.contains("Failed to generate solution")
    );
    let _ = std::fs::remove_file("test_ai.rs");
}

#[tokio::test]
async fn test_handle_autofix_sensitive_file() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    let payload: AutoFixPayload = serde_json::from_value(serde_json::json!({
        "file_path": "Cargo.toml",
        "line": 10,
        "error_message": "some error"
    }))
    .unwrap();

    let res = handle_autofix(ConnectInfo(addr), Json(payload)).await;
    let res_body = axum::response::IntoResponse::into_response(res);
    let bytes = axum::body::to_bytes(res_body.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(body_str.contains("Access denied"));
}

#[tokio::test]
async fn test_handle_autofix_wrong_extension() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    let _ = std::fs::write("test_autofix.txt", "SECRET=1");
    let payload: AutoFixPayload = serde_json::from_value(serde_json::json!({
        "file_path": "test_autofix.txt",
        "line": 10,
        "error_message": "some error"
    }))
    .unwrap();

    let res = handle_autofix(ConnectInfo(addr), Json(payload)).await;
    let res_body = axum::response::IntoResponse::into_response(res);
    let bytes = axum::body::to_bytes(res_body.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(body_str.contains("Autofix is restricted"));
    let _ = std::fs::remove_file("test_autofix.txt");
}

#[tokio::test]
async fn test_handle_autofix_valid_file_ai_offline() {
    let _guard = ENV_LOCK.lock().await;
    unsafe {
        std::env::set_var("RULLST_AI_PROVIDER", "invalid_provider");
    }
    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    let _ = std::fs::write("test_autofix_offline.rs", "fn main() {}");
    let payload: AutoFixPayload = serde_json::from_value(serde_json::json!({
        "file_path": "test_autofix_offline.rs",
        "line": 1,
        "error_message": "some error"
    }))
    .unwrap();

    let res = handle_autofix(ConnectInfo(addr), Json(payload)).await;
    let res_body = axum::response::IntoResponse::into_response(res);
    let bytes = axum::body::to_bytes(res_body.into_body(), 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(body_str.contains("\"success\":false"));
    let _ = std::fs::remove_file("test_autofix_offline.rs");
}
