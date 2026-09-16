//! Real middleware/form contracts; no live provider, browser or deployment claim.
use axum::{
    Extension, Form, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::Html,
    routing::get,
};
use rullst_core::{
    config::{Environment, SecurityConfig},
    security::{CsrfToken, apply_security_baseline, csrf_middleware},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;

#[derive(serde::Deserialize)]
struct Message {
    message: String,
}

fn app(calls: Arc<AtomicUsize>) -> Router {
    let router = Router::new()
        .route(
            "/messages",
            get(|Extension(csrf): Extension<CsrfToken>| async move {
                Html(format!(
                    "<form method=\"post\" hx-post=\"/messages\"><input type=\"hidden\" name=\"_token\" value=\"{}\" /></form>",
                    rullst_core::html::escape_str(csrf.as_str())
                ))
            })
            .post(move |Form(message): Form<Message>| async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Html(rullst_core::html::escape_str(&message.message).into_owned())
            }),
        )
        // Form-enabled local apps may already have this layer. The production
        // Server must not replace its token with a second, divergent token.
        .layer(axum::middleware::from_fn(csrf_middleware));
    apply_security_baseline(router, SecurityConfig::default(), Environment::Production)
        .expect("valid production policy")
}

async fn first_visit(app: &Router) -> (String, String) {
    let response = app
        .clone()
        .oneshot(
            Request::get("/messages")
                .header(header::USER_AGENT, "Mozilla/5.0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookies: Vec<_> = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .collect();
    assert_eq!(
        cookies.len(),
        1,
        "nested layers must emit exactly one cookie"
    );
    let cookie = cookies[0].to_str().unwrap();
    assert!(cookie.contains("; Secure"));
    assert!(cookie.contains("; SameSite=Lax"));
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let pair = cookie.split(';').next().unwrap().to_owned();
    let token = pair.strip_prefix("rullst_csrf=").unwrap().to_owned();
    let html = to_bytes(response.into_body(), 8192).await.unwrap();
    assert!(String::from_utf8_lossy(&html).contains(&format!("value=\"{token}\"")));
    (pair, token)
}

fn post(cookie: Option<&str>, token: Option<&str>, body: String, htmx: bool) -> Request<Body> {
    let mut request = Request::post("/messages")
        .header(header::USER_AGENT, "Mozilla/5.0")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if let Some(token) = token {
        request = request.header("X-CSRF-Token", token);
    }
    if htmx {
        request = request.header("HX-Request", "true");
    }
    request.body(Body::from(body)).unwrap()
}

#[tokio::test]
async fn normal_and_htmx_forms_and_header_tokens_preserve_the_submitted_message() {
    let calls = Arc::new(AtomicUsize::new(0));
    let app = app(calls.clone());
    let (cookie, token) = first_visit(&app).await;
    for (htmx, use_header) in [(false, false), (true, false), (true, true)] {
        let body = if use_header {
            "message=Rust+%26+HTMX".to_owned()
        } else {
            format!("message=Rust+%26+HTMX&_token={token}")
        };
        let response = app
            .clone()
            .oneshot(post(
                Some(&cookie),
                use_header.then_some(token.as_str()),
                body,
                htmx,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), 8192).await.unwrap(),
            "Rust &amp; HTMX"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn htmx_missing_cookie_or_invalid_tokens_never_reaches_the_handler() {
    let calls = Arc::new(AtomicUsize::new(0));
    let app = app(calls.clone());
    let (cookie, token) = first_visit(&app).await;
    let cases = [
        (
            None,
            Some(token.as_str()),
            format!("message=hello&_token={token}"),
            "CSRF token cookie missing",
        ),
        (
            Some(cookie.as_str()),
            None,
            "message=hello".to_owned(),
            "Invalid or missing CSRF token",
        ),
        (
            Some(cookie.as_str()),
            None,
            "message=hello&_token=wrong".to_owned(),
            "Invalid or missing CSRF token",
        ),
        (
            Some(cookie.as_str()),
            Some("wrong"),
            format!("message=hello&_token={token}"),
            "Invalid CSRF token",
        ),
    ];
    for (cookie, header, body, error) in cases {
        let response = app
            .clone()
            .oneshot(post(cookie, header, body, true))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(to_bytes(response.into_body(), 8192).await.unwrap(), error);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
