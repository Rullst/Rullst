//! The pinned Alpine.js CSP build (`@alpinejs/csp`) that `cargo rullst add
//! alpine` vendors. Bumping it changes this file only.
//!
//! It follows the cooldown rule of `tailwind_release.rs`: the newest release
//! published at least 14 days before the pin (3.17.5, published 2026-10-20,
//! pinned 2026-11-10).
//!
//! Source (3.17.5, published 2026-10-20): the npm registry document
//! `https://registry.npmjs.org/@alpinejs%2fcsp`, `versions["3.17.5"].dist`
//! (`tarball` and `integrity`, the Subresource Integrity form of the
//! tarball's SHA-512), checked on 2026-11-10 by downloading the tarball and
//! hashing it (`openssl dgst -sha512 -binary | base64`).

/// The `@alpinejs/csp` version.
pub(crate) const VERSION: &str = "3.17.5";

/// The registry tarball of [`VERSION`].
pub(crate) const TARBALL: &str = "https://registry.npmjs.org/@alpinejs/csp/-/csp-3.17.5.tgz";

/// `dist.integrity` of [`VERSION`] in the npm registry.
pub(crate) const INTEGRITY: &str = "sha512-7e5lYgzJJL2eZ7esnx+E7wtxws+VDs9em9ztttZ+AQAYEng2bABHFp2SUga0wt84RyfL7hc6B+S/RGglXsPf6w==";

/// The browser build inside the tarball: it sets `window.Alpine` and starts
/// Alpine in a microtask after it runs.
pub(crate) const ENTRY: &str = "package/dist/cdn.min.js";

/// Largest tarball accepted (the 3.17.5 tarball is 173 kB).
pub(crate) const MAX_TARBALL_BYTES: u64 = 4 * 1024 * 1024;

/// Largest [`ENTRY`] accepted (72 kB in 3.17.5).
pub(crate) const MAX_ENTRY_BYTES: u64 = 2 * 1024 * 1024;

/// The first line of the vendored file: names the version and keeps the MIT
/// notice the minified build lacks (Alpine is MIT, (c) Caleb Porzio and
/// contributors; the bundled `@vue/reactivity` notice follows the code).
pub(crate) const BANNER: &str = "/*! @alpinejs/csp 3.17.5 | MIT License | (c) Caleb Porzio and contributors | https://github.com/alpinejs/alpine/blob/main/LICENSE.md */";
