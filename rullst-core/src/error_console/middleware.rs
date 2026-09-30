//! Development middleware catching application panic unwinds.

use crate::error_console::renderer::render_console_html;
use axum::{
    body::Body,
    extract::{ConnectInfo, connect_info::MockConnectInfo},
    http::{Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;

/// Middleware that catches panic unwinds in dev mode and presents the Self-Healing Console.
///
/// Panic details are rendered only when the request's [`ConnectInfo`] peer is a
/// loopback address, or when no peer metadata exists (in-process dispatch such
/// as [`crate::testing::TestApp`]). Any other peer receives a plain `500` with no
/// panic payload or backtrace. `Server` mounts this middleware only in debug
/// builds running in Development.
#[cfg_attr(mutants, mutants::skip)]
pub async fn catch_panic_middleware(req: Request<Body>, next: Next) -> Response {
    // Same lookup order as the `ConnectInfo` extractor used by `/_rullst/*`.
    let extensions = req.extensions();
    let peer = extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| *peer)
        .or_else(|| {
            extensions
                .get::<MockConnectInfo<SocketAddr>>()
                .map(|MockConnectInfo(peer)| *peer)
        });
    let render_details = peer.is_none_or(|peer| peer.ip().to_canonical().is_loopback());
    let handle = tokio::spawn(async move { next.run(req).await });

    match handle.await {
        Ok(response) => response,
        Err(join_err) => {
            if join_err.is_panic() && render_details {
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
