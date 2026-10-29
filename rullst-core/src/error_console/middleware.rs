//! Development middleware catching application panic unwinds.

use crate::error_console::capture::spawn_capturing;
use crate::error_console::renderer::render_console_html;
use axum::{
    body::Body,
    extract::{ConnectInfo, connect_info::MockConnectInfo},
    http::{Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;

/// Request marker that `Server` adds where it also mounts the loopback-only
/// `GET /_rullst/errors/{id}` route. Only then does the console record a panic
/// and name its context path, so the page never points to an absent route.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ErrorContextServed;

/// Middleware that catches panic unwinds in dev mode and presents the Self-Healing Console.
///
/// The page's "Ask your AI assistant" panel holds a prompt to copy into the
/// developer's own assistant. Where `Server` also serves
/// `GET /_rullst/errors/{id}` (loopback only), each rendered panic is recorded
/// under a random id (see `store`) and the panel names that path.
///
/// Panic details are rendered only when the request's [`ConnectInfo`] peer is a
/// loopback address, or when no peer metadata exists (in-process dispatch such
/// as [`crate::testing::TestApp`]). Any other peer receives a plain `500` with no
/// panic payload or backtrace. `Server` mounts this middleware only in debug
/// builds running in Development.
#[cfg_attr(mutants, mutants::skip)]
pub async fn catch_panic_middleware(mut req: Request<Body>, next: Next) -> Response {
    let render_details = console_details_allowed(req.extensions());
    // Shared with the security headers middleware, which reuses a nonce that
    // is already present, so the console's inline style and script match the
    // emitted CSP.
    let nonce = crate::security::CspNonce::get_or_insert(req.extensions_mut());
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let record = req.extensions().get::<ErrorContextServed>().is_some();
    let (handle, panic_slot) = spawn_capturing(async move { next.run(req).await });

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

                let capture = panic_slot.take();
                // Only the console's own view of the panic is kept: no query
                // string, headers, cookies or body.
                let error_id = record.then(|| {
                    crate::error_console::store::record(
                        &message,
                        capture.location.clone(),
                        capture.backtrace.as_deref(),
                        &method,
                        &path,
                    )
                });
                let html_content = render_console_html(
                    &message,
                    &capture,
                    Some((&method, &path)),
                    Some(nonce.as_str()),
                    error_id.as_deref(),
                )
                .await;

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

/// Whether panic details may be shown for a request: its peer is loopback, or
/// no peer metadata exists (in-process dispatch such as `TestApp`).
pub(crate) fn console_details_allowed(extensions: &axum::http::Extensions) -> bool {
    // Same lookup order as the `ConnectInfo` extractor used by `/_rullst/*`.
    let peer = extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| *peer)
        .or_else(|| {
            extensions
                .get::<MockConnectInfo<SocketAddr>>()
                .map(|MockConnectInfo(peer)| *peer)
        });
    peer.is_none_or(|peer| peer.ip().to_canonical().is_loopback())
}
