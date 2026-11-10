//! The `static/` mount and the `Cache-Control` of its responses.
//!
//! A file whose name carries a content hash (`app.3f9a2c1b.css`,
//! `chunk-3f9a2c1b.js`) gets new bytes only under a new name, so browsers and
//! CDNs may keep it for a year without asking again. Every other static file
//! may be stored but is revalidated before each use (`no-cache`): `ServeDir`
//! sends `ETag` and `Last-Modified` and answers a matching `If-None-Match` or
//! `If-Modified-Since` with `304 Not Modified`. Only `2xx` and `304` responses
//! are labelled, so errors keep the security baseline's `no-store`, which it
//! adds only when a response has no `Cache-Control` yet.
//!
//! [`crate::assets::url`] links `app.css` as `app.<hash>.css`; the mount
//! resolves that name to the real file before any of the above (see
//! `static_hashed.rs`).

use crate::assets::{Limits, Manifest};
use crate::server::server_middleware::zstd_static_from;
use axum::http::{HeaderValue, StatusCode, header};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[path = "static_hashed.rs"]
mod hashed;

/// For fingerprinted files: one year, never revalidated.
pub(crate) const IMMUTABLE: &str = "public, max-age=31536000, immutable";
/// For every other static file: store, but revalidate with the validators.
pub(crate) const REVALIDATE: &str = "no-cache";

/// Serves `<working directory>/static` under `/static` and makes its
/// manifest the one [`crate::assets::url`] reads.
pub(crate) fn mount_static_assets(app: axum::Router) -> axum::Router {
    let root = Path::new(".");
    let manifest = static_manifest(root);
    crate::assets::activate(Arc::clone(&manifest));
    mount_static_assets_with(app, root, manifest)
}

/// The manifest of `root/static`; debug builds follow edits.
fn static_manifest(root: &Path) -> Arc<Manifest> {
    Arc::new(Manifest::build(
        root.join("static"),
        cfg!(debug_assertions),
        Limits::DEFAULT,
    ))
}

/// Serves `root/static` under `/static` with its own manifest, without
/// changing the one [`crate::assets::url`] reads.
#[cfg(test)]
pub(crate) fn mount_static_assets_at(app: axum::Router, root: &Path) -> axum::Router {
    let manifest = static_manifest(root);
    mount_static_assets_with(app, root, manifest)
}

/// Serves `root/static` under `/static`: Brotli siblings through `ServeDir`,
/// Zstandard siblings through [`zstd_static_from`], the cache policy of the
/// original request path around both, and outermost the content-hashed names
/// of `manifest` (see [`hashed::hashed_static_middleware`]).
fn mount_static_assets_with(
    app: axum::Router,
    root: &Path,
    manifest: Arc<Manifest>,
) -> axum::Router {
    let root: PathBuf = root.to_path_buf();
    let directory = root.join("static");
    app.nest_service(
        "/static",
        tower_http::services::ServeDir::new(directory).precompressed_br(),
    )
    .layer(axum::middleware::from_fn(
        move |req: axum::extract::Request, next: axum::middleware::Next| {
            let root = root.clone();
            async move { zstd_static_from(&root, req, next).await }
        },
    ))
    .layer(axum::middleware::from_fn(static_cache_middleware))
    .layer(axum::middleware::from_fn(
        move |req: axum::extract::Request, next: axum::middleware::Next| {
            let manifest = Arc::clone(&manifest);
            async move { hashed::hashed_static_middleware(manifest, req, next).await }
        },
    ))
}

/// Sets the policy of [`static_cache_policy`] on successful and `304`
/// responses below `/static/`.
pub(crate) async fn static_cache_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let policy = static_cache_policy(req.uri().path());
    let mut response = next.run(req).await;
    let status = response.status();
    if let Some(policy) = policy
        && (status.is_success() || status == StatusCode::NOT_MODIFIED)
    {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(policy));
    }
    response
}

/// The `Cache-Control` value for a request path below `/static/`, or `None`
/// for any other path.
pub(crate) fn static_cache_policy(path: &str) -> Option<&'static str> {
    let relative = path.strip_prefix("/static/")?;
    let name = relative.rsplit('/').next().unwrap_or(relative);
    Some(if is_fingerprinted(name) {
        IMMUTABLE
    } else {
        REVALIDATE
    })
}

/// Whether a file name carries a content hash: after a non-empty first part,
/// a `.`- or `-`-separated segment before the final extension holds 8 to 64
/// lowercase hexadecimal characters with at least one digit and one letter
/// (`app.3f9a2c1b.css`, `chunk-3f9a2c1b.js`, `app.3f9a2c1b.js.map`). Dates
/// such as `report-20261008.pdf` and plain words do not count.
pub(crate) fn is_fingerprinted(name: &str) -> bool {
    let Some((stem, _extension)) = name.rsplit_once('.') else {
        return false;
    };
    let mut segments = stem.split(['.', '-']);
    if segments.next().is_none_or(str::is_empty) {
        return false;
    }
    segments.any(is_content_hash)
}

fn is_content_hash(segment: &str) -> bool {
    let bytes = segment.as_bytes();
    (8..=64).contains(&bytes.len())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        && bytes.iter().any(u8::is_ascii_digit)
        && bytes.iter().any(u8::is_ascii_lowercase)
}

#[cfg(test)]
#[path = "static_cache_tests.rs"]
mod tests;
