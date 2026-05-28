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

use std::sync::Mutex;
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_error_console_catches_panic_and_renders_html() {
    let _guard = ENV_LOCK.lock().unwrap();
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
