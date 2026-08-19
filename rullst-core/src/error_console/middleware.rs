//! Development middleware catching application panic unwinds.

use crate::error_console::renderer::render_console_html;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Middleware that catches panic unwinds in dev mode and presents the Self-Healing Console.
#[cfg_attr(mutants, mutants::skip)]
pub async fn catch_panic_middleware(req: Request<Body>, next: Next) -> Response {
    let handle = tokio::spawn(async move { next.run(req).await });

    match handle.await {
        Ok(response) => response,
        Err(join_err) => {
            if join_err.is_panic() {
                let panic_payload = join_err.into_panic();
                let message = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unhandled application panic".to_string()
                };

                let backtrace = std::backtrace::Backtrace::capture();
                let html_content = render_console_html(&message, &backtrace).await;

                match Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(html_content))
                {
                    Ok(res) => res,
                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                }
            } else {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
