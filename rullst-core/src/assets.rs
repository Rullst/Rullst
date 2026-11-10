//! Content-hashed URLs for the files the server publishes under `/static`.
//!
//! [`url`] turns a path relative to the project's `static/` directory into
//! the URL a page should link:
//!
//! ```
//! # extern crate rullst_core as rullst;
//! use rullst::html;
//!
//! let head = html! {
//!     <link rel="stylesheet" href={rullst::assets::url("app.css")} />
//!     <script defer src={rullst::assets::url("js/app.js")}></script>
//! };
//! // Outside a running server (here, a doc test) the URLs are the plain ones.
//! assert_eq!(
//!     head,
//!     "<link rel=\"stylesheet\" href=\"/static/app.css\" />\
//!      <script defer src=\"/static/js/app.js\"></script>"
//! );
//! ```
//!
//! While a [`Server`](crate::Server) serves `static/`, the same call returns
//! `/static/app.<hash>.css`, where `<hash>` is the first 16 hexadecimal
//! characters of the SHA-256 of the file's content. The static mount resolves
//! that name to `static/app.css` and serves it with
//! `Cache-Control: public, max-age=31536000, immutable`, so browsers and CDNs
//! keep it until the content, and with it the URL, changes. The plain
//! `/static/app.css` keeps working with `no-cache`.
//!
//! - **When the hash is computed.** The server hashes the files under
//!   `static/` when it mounts the directory (at most 10,000 files of up to
//!   32 MiB each, 16 directory levels deep). Debug builds follow edits: each
//!   call compares the file's size and modification time with the recorded
//!   ones and hashes it again when they differ, and a file added after start
//!   is hashed on its first call, so `cargo rullst dev` never links an old
//!   stylesheet. Release builds keep the start-up hashes for links.
//! - **An outdated hash.** A request for a hash that no longer matches the
//!   file (a page cached before a deploy) is answered with
//!   `307 Temporary Redirect` to the current URL and `Cache-Control:
//!   no-store`; the new content is never served as immutable under the old
//!   name. The server checks the file's size and modification time on every
//!   hashed request, in every build.
//! - **Plain URLs.** [`url`] returns `/static/<path>` unchanged when no server
//!   has mounted `static/` in this process (unit tests, a function used
//!   outside a server), when the file does not exist, is a symbolic link or
//!   lies below one, exceeds the limits above, has no extension, is a `.br`
//!   or `.zst` sibling, already carries a content hash (`app.3f9a2c1b.css`),
//!   or when its path has a segment that is empty, `.`/`..`, starts with `.`
//!   or uses characters other than ASCII letters, digits, `-`, `_`, `.` and
//!   `~` (such names would need percent-encoding).
//! - **Unchanged rules.** Hashed names are resolved before the static
//!   service, which still serves the file: Brotli and Zstandard siblings
//!   (`app.css.br`, `app.css.zst`), path-traversal checks and the handling of
//!   symbolic links stay those of the `/static` mount.
//!
//! The path is relative to `static/` (`"app.css"`, `"css/site.css"`);
//! leading `/` characters are ignored. Added in 13.0.0-alpha.5.

mod manifest;
mod names;
#[cfg(test)]
mod tests;

pub(crate) use manifest::{Limits, Manifest, Resolution};
pub(crate) use names::looks_hashed;

use std::sync::{Arc, PoisonError, RwLock};

/// The manifest of the `static/` directory the server mounted last.
static ACTIVE: RwLock<Option<Arc<Manifest>>> = RwLock::new(None);

/// The URL of `path`, a file below the `static/` directory: the
/// content-hashed `/static/app.<hash>.css` while a server serves that
/// directory, else the plain `/static/app.css`. See the [module
/// documentation](self) for when each form is returned.
///
/// ```
/// # extern crate rullst_core as rullst;
/// // No server has mounted `static/` in a doc test.
/// assert_eq!(rullst::assets::url("css/site.css"), "/static/css/site.css");
/// assert_eq!(rullst::assets::url(String::from("/app.js")), "/static/app.js");
/// ```
pub fn url(path: impl AsRef<str>) -> String {
    let path = path.as_ref().trim_start_matches('/');
    let active = ACTIVE
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    match active {
        Some(manifest) => manifest.url(path),
        None => names::plain_url(path),
    }
}

/// Makes `manifest` the one [`url`] reads, replacing an earlier one.
pub(crate) fn activate(manifest: Arc<Manifest>) {
    *ACTIVE.write().unwrap_or_else(PoisonError::into_inner) = Some(manifest);
}

/// Serializes the tests that replace the process-wide manifest.
#[cfg(test)]
pub(crate) static TEST_MANIFEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Forgets the active manifest (tests only).
#[cfg(test)]
pub(crate) fn deactivate() {
    *ACTIVE.write().unwrap_or_else(PoisonError::into_inner) = None;
}
