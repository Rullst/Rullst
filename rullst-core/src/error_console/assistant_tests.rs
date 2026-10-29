//! The "Ask your AI assistant" panel: its prompt quotes only the page, is
//! escaped, names the recorded context only where the server serves it, and
//! never names the removed `cargo rullst ai` command.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::assistant::{PROMPT_FRAMES, PromptSource, prompt};
use super::capture::PanicCapture;
use super::middleware::{ErrorContextServed, catch_panic_middleware};
use super::renderer::render_console_html;
use tower::ServiceExt;

const PANIC_LINE: u32 = line!() + 3;

async fn panics() {
    panic!("assistant probe panic");
}

/// The error page for `GET /panic?…` with secrets in the query, headers and
/// body, with or without the marker `Server` adds next to its context route.
async fn error_page(context_served: bool) -> String {
    let mut router = axum::Router::new()
        .route("/panic", axum::routing::get(panics))
        .layer(axum::middleware::from_fn(catch_panic_middleware));
    if context_served {
        router = router.layer(axum::Extension(ErrorContextServed));
    }
    let request = axum::http::Request::get("/panic?token=query-secret-value")
        .header("authorization", "Bearer header-secret-value")
        .header("cookie", "session=cookie-secret-value")
        .body(axum::body::Body::from("password=body-secret-value"))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status().as_u16(), 500);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    String::from_utf8_lossy(&body).into_owned()
}

/// The text of the prompt `<textarea>`, unescaped.
fn prompt_text(page: &str) -> String {
    let start = page
        .split("<textarea id=\"ai-prompt\"")
        .nth(1)
        .and_then(|rest| rest.split_once('>'))
        .map(|(_, text)| text)
        .expect("prompt textarea");
    let escaped = start.split("</textarea>").next().expect("closed textarea");
    escaped
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&amp;", "&")
}

#[tokio::test]
async fn error_page_offers_an_assistant_prompt_not_the_removed_cli() {
    let page = error_page(true).await;
    assert!(
        !page.contains("cargo rullst ai"),
        "the removed CLI is named"
    );
    assert!(page.contains("Ask your AI assistant"));
    assert!(page.contains("<code>cargo rullst generate:ai-context</code>"));

    let prompt = prompt_text(&page);
    assert!(
        prompt.contains("Error: assistant probe panic\n"),
        "{prompt}"
    );
    assert!(
        prompt.contains(&format!(
            "src/error_console/assistant_tests.rs:{PANIC_LINE}\n"
        )),
        "{prompt}"
    );
    assert!(prompt.contains("\nRequest: GET /panic\n"), "{prompt}");
    assert!(
        prompt.contains("Follow the project's AGENTS.md."),
        "{prompt}"
    );

    // The prompt and the panel name the recorded context, which holds the
    // same error.
    let id = prompt
        .split("GET /_rullst/errors/")
        .nth(1)
        .map(|rest| {
            rest.chars()
                .take_while(char::is_ascii_hexdigit)
                .collect::<String>()
        })
        .expect("recorded-context path in the prompt");
    assert!(super::store::valid_id(&id), "{id}");
    assert!(page.contains(&format!(
        "<code class=\"context-url\">/_rullst/errors/{id}</code>"
    )));
    let context = super::store::lookup(&id).expect("recorded context");
    assert_eq!(context.message, "assistant probe panic");
    assert_eq!(context.path, "/panic");

    // Nothing the page does not show: no query string, headers or body.
    for secret in [
        "query-secret",
        "header-secret",
        "cookie-secret",
        "body-secret",
    ] {
        assert!(!page.contains(secret), "{secret} reached the error page");
    }
}

#[tokio::test]
async fn without_the_context_route_the_page_names_no_context_path() {
    let page = error_page(false).await;
    assert!(
        !page.contains("cargo rullst ai"),
        "the removed CLI is named"
    );
    assert!(!page.contains("/_rullst/errors/"), "no route serves it");
    assert!(page.contains("This server does not record errors"));
    let prompt = prompt_text(&page);
    assert!(
        prompt.contains("Error: assistant probe panic\n"),
        "{prompt}"
    );
    assert!(prompt.contains("\nRequest: GET /panic\n"), "{prompt}");
}

const BACKTRACE: &str = "   0: std::panicking::begin_panic\n             at /rustc/abc/library/std/src/panicking.rs:689:12\n   1: tokio::runtime::task::harness::poll\n             at /home/dev/.cargo/registry/src/index.crates.io-1/tokio-1.52.3/src/runtime/task/harness.rs:473:19\n   2: app::controllers::users::show\n             at ./src/controllers/users.rs:42:9\n   3: app::main\n             at ./src/main.rs:7:5\n";

#[tokio::test]
async fn the_prompt_is_escaped_inside_the_textarea() {
    let capture = PanicCapture {
        location: Some(("src\\controllers\\users.rs".to_string(), 42)),
        backtrace: Some(BACKTRACE.to_string()),
    };
    let message = "</textarea><script>alert(1)</script>";
    let page =
        render_console_html(message, &capture, Some(("GET", "/users/<b>")), None, None).await;
    assert!(!page.contains("<script>alert(1)"), "unescaped page text");
    assert_eq!(page.matches("</textarea>").count(), 1);
    assert!(page.contains("Request: GET /users/&lt;b&gt;"));
    let prompt = prompt_text(&page);
    assert!(prompt.contains(&format!("Error: {message}\n")), "{prompt}");
    assert!(
        prompt.contains("Location: src/controllers/users.rs:42\n"),
        "{prompt}"
    );
}

#[test]
fn the_prompt_quotes_only_the_first_project_frames() {
    let source = |backtrace| PromptSource {
        message: "boom",
        location: None,
        backtrace,
        request: None,
        error_id: None,
    };
    let text = prompt(&source(BACKTRACE));
    assert!(text.contains("Location: unknown\n"), "{text}");
    assert!(!text.contains("Request:"), "{text}");
    assert!(!text.contains("/_rullst/errors/"), "{text}");
    assert!(text.contains("- app::controllers::users::show at ./src/controllers/users.rs:42:9\n"));
    assert!(text.contains("- app::main at ./src/main.rs:7:5\n"));
    assert!(!text.contains("std::panicking") && !text.contains("tokio::runtime"));

    let long: String = (0..PROMPT_FRAMES + 3)
        .map(|frame| format!("  {frame}: app::f{frame}\n             at ./src/f.rs:{frame}:1\n"))
        .collect();
    let text = prompt(&source(&long));
    assert_eq!(
        text.lines().filter(|line| line.starts_with("- ")).count(),
        PROMPT_FRAMES
    );

    assert!(prompt(&source("")).contains("Stack trace: not captured (set RUST_BACKTRACE=1"));
    let dependencies_only = "   0: std::panicking::begin_panic\n             at /rustc/abc/library/std/src/panicking.rs:689:12\n";
    assert!(
        prompt(&source(dependencies_only))
            .contains("Stack trace: no frame from the project's own code was captured.")
    );
}
