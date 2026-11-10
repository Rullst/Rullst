//! Content-hashed names below `/static/` (`app.<hash>.css`, written by
//! [`crate::assets::url`]): resolved to the real file and served immutable,
//! or redirected when the hash is outdated.

use crate::assets::{Manifest, Resolution, looks_hashed};
use crate::server::server_middleware::static_asset_path;
use axum::http::{HeaderValue, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

/// For a `GET` or `HEAD` of a hashed name of a file in `manifest`:
///
/// - the current hash: the request continues for the real path (so the
///   static service, its Brotli/Zstandard siblings and validators serve it)
///   and a `2xx` or `304` answer gets the immutable policy;
/// - an outdated hash: `307 Temporary Redirect` to the current URL with
///   `Cache-Control: no-store`, so no cache keeps the new content under the
///   old name.
///
/// Every other request, including a path that `static_asset_path` refuses
/// (`..`, `.`, empty segments, backslashes, `%`) and a real file whose name
/// looks hashed, continues unchanged.
pub(crate) async fn hashed_static_middleware(
    manifest: Arc<Manifest>,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if !matches!(*req.method(), Method::GET | Method::HEAD) {
        return next.run(req).await;
    }
    let Some(requested) = static_asset_path(req.uri().path())
        .filter(|requested| looks_hashed(requested))
        .map(str::to_owned)
    else {
        return next.run(req).await;
    };
    // Metadata checks and hashing are blocking file-system work.
    let resolution = tokio::task::spawn_blocking(move || manifest.resolve(&requested))
        .await
        .unwrap_or(Resolution::PassThrough);
    match resolution {
        Resolution::PassThrough => next.run(req).await,
        Resolution::Outdated(location) => outdated(&location),
        Resolution::Current(real) => {
            let Ok(uri) = format!("/static/{real}").parse::<Uri>() else {
                return next.run(req).await;
            };
            *req.uri_mut() = uri;
            let mut response = next.run(req).await;
            let status = response.status();
            if status.is_success() || status == StatusCode::NOT_MODIFIED {
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static(super::IMMUTABLE),
                );
            }
            response
        }
    }
}

/// The redirect for an outdated hash.
fn outdated(location: &str) -> Response {
    let Ok(location) = HeaderValue::from_str(location) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    (
        StatusCode::TEMPORARY_REDIRECT,
        [
            (header::LOCATION, location),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
    )
        .into_response()
}
