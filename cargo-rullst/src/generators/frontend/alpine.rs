//! Vendoring the pinned Alpine.js CSP build: the registry tarball is
//! downloaded, checked against the pinned `dist.integrity` (SHA-512), and
//! only then is its browser build read from it, in memory. Nothing from the
//! tarball is unpacked to disk and nothing in it runs.

use super::alpine_release::{BANNER, ENTRY, MAX_ENTRY_BYTES, MAX_TARBALL_BYTES, TARBALL};
use super::fetch::{FetchError, Fetcher};
use base64::Engine as _;
use sha2::{Digest, Sha512};
use std::io::Read;

/// Why the CSP build could not be vendored.
#[derive(Debug, thiserror::Error)]
pub(crate) enum VendorError {
    #[error("downloading @alpinejs/csp failed: {0}")]
    Download(#[from] FetchError),
    #[error(
        "the @alpinejs/csp tarball does not have the pinned integrity (expected {expected}, found {actual}); nothing was written"
    )]
    IntegrityMismatch { expected: String, actual: String },
    #[error("the verified @alpinejs/csp tarball could not be read: {0}")]
    Archive(String),
    #[error("the verified @alpinejs/csp tarball has no {ENTRY} within {MAX_ENTRY_BYTES} bytes")]
    MissingEntry,
}

/// The Subresource Integrity string (`sha512-<base64>`) of `bytes`.
pub(crate) fn integrity(bytes: &[u8]) -> String {
    format!(
        "sha512-{}",
        base64::engine::general_purpose::STANDARD.encode(Sha512::digest(bytes))
    )
}

/// The content of `static/vendor/alpine-csp.min.js`: [`BANNER`], then the
/// browser build of `tarball`, once the tarball's integrity is `expected`.
pub(crate) fn vendored_with(tarball: &[u8], expected: &str) -> Result<String, VendorError> {
    let actual = integrity(tarball);
    if actual != expected {
        return Err(VendorError::IntegrityMismatch {
            expected: expected.to_owned(),
            actual,
        });
    }
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tarball));
    let entries = archive
        .entries()
        .map_err(|error| VendorError::Archive(error.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|error| VendorError::Archive(error.to_string()))?;
        let is_entry = entry
            .path()
            .map(|path| path.to_str() == Some(ENTRY))
            .unwrap_or(false);
        if !is_entry || !entry.header().entry_type().is_file() {
            continue;
        }
        let mut code = String::new();
        entry
            .take(MAX_ENTRY_BYTES + 1)
            .read_to_string(&mut code)
            .map_err(|error| VendorError::Archive(error.to_string()))?;
        if code.len() as u64 > MAX_ENTRY_BYTES {
            return Err(VendorError::MissingEntry);
        }
        return Ok(format!("{BANNER}\n{code}"));
    }
    Err(VendorError::MissingEntry)
}

/// Downloads the pinned tarball and returns its browser build once the
/// tarball's integrity is `expected` ([`super::alpine_release::INTEGRITY`] outside tests).
pub(crate) fn download_with<F: Fetcher>(
    fetcher: &F,
    expected: &str,
) -> Result<String, VendorError> {
    let mut tarball = Vec::new();
    fetcher.fetch(TARBALL, MAX_TARBALL_BYTES, &mut tarball)?;
    vendored_with(&tarball, expected)
}

/// Whether `existing` (a vendored file) is the pinned version: its first line
/// is [`BANNER`].
pub(crate) fn is_pinned(existing: &str) -> bool {
    existing.lines().next() == Some(BANNER)
}
