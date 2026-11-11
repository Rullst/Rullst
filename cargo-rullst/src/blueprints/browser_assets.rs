//! Browser assets vendored into the `static/` directory of HTML scaffolds,
//! served from the application's own origin so pages need no CDN and run
//! under the strict `script-src 'self'` policy. `assets/README.md` records
//! their provenance.

use std::fs;
use std::path::Path;

/// htmx 1.9.12, `dist/htmx.min.js` of commit
/// `f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d` plus a terminal LF.
/// SHA-256 `73eabc44d978b226a667c62ca3c40e99236d11aa6f8fc8a27be6f0b36a73b42d`.
pub(crate) const HTMX: &[u8] = include_bytes!("assets/htmx-1.9.12.min.js");

/// The Server-Sent Events extension of the same htmx 1.9.12 release,
/// patched: upstream
/// <https://github.com/bigskysoftware/htmx/blob/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/dist/ext/sse.js>
/// (raw: <https://raw.githubusercontent.com/bigskysoftware/htmx/f38e07d4be8145a39e2bb477ec9fcc56bdd2d16d/dist/ext/sse.js>,
/// SHA-256 `be05b2e2265279f035271adbea0b72a356f20ce4dfa5870481bfe9c51b822fc1`)
/// computes its reconnect delay with the bitwise XOR `2 ^ retryCount`, so the
/// delay never grows past about 2.5 seconds and is sometimes zero. The copy
/// uses `Math.pow(2, retryCount)`, resets the count when a connection opens,
/// and says so in its header comment; `assets/README.md` has the diff.
/// SHA-256 `afb588b88863ec49fe7d9f958b788ab1e1df1f45016466d2ea573f374c89e5d4`.
pub(crate) const HTMX_SSE: &[u8] = include_bytes!("assets/htmx-ext-sse-1.9.12.js");

/// The Zero-Clause BSD license that covers both htmx files.
pub(crate) const HTMX_LICENSE: &[u8] = include_bytes!("assets/HTMX-LICENSE");

/// Every vendored file, by its name under `static/`.
pub(crate) const STATIC_FILES: [(&str, &[u8]); 3] = [
    ("htmx-1.9.12.min.js", HTMX),
    ("htmx-ext-sse-1.9.12.js", HTMX_SSE),
    ("HTMX-LICENSE", HTMX_LICENSE),
];

/// Writes every vendored file into `static_dir`.
pub(crate) fn write_static(static_dir: &Path) -> std::io::Result<()> {
    for (name, bytes) in STATIC_FILES {
        fs::write(static_dir.join(name), bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    /// The digests recorded above and in `assets/README.md`.
    const HTMX_SHA256: &str = "73eabc44d978b226a667c62ca3c40e99236d11aa6f8fc8a27be6f0b36a73b42d";
    const HTMX_SSE_SHA256: &str =
        "afb588b88863ec49fe7d9f958b788ab1e1df1f45016466d2ea573f374c89e5d4";
    /// Upstream `dist/ext/sse.js`, before the backoff patch.
    const UPSTREAM_HTMX_SSE_SHA256: &str =
        "be05b2e2265279f035271adbea0b72a356f20ce4dfa5870481bfe9c51b822fc1";

    fn sha256(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    #[test]
    fn vendored_files_keep_their_recorded_digests() {
        assert_eq!(sha256(HTMX), HTMX_SHA256);
        assert_eq!(sha256(HTMX_SSE), HTMX_SSE_SHA256);
        let readme = include_str!("assets/README.md");
        let source = include_str!("browser_assets.rs");
        for digest in [HTMX_SHA256, HTMX_SSE_SHA256, UPSTREAM_HTMX_SSE_SHA256] {
            assert!(readme.contains(digest), "{digest}");
            // The item documentation and the constant in this test.
            assert_eq!(source.matches(digest).count(), 2, "{digest}");
        }
    }

    #[test]
    fn the_sse_extension_backs_off_exponentially_and_resets_on_open() {
        let source = std::str::from_utf8(HTMX_SSE).expect("UTF-8 script");
        assert!(source.contains("Patched by Rullst"));
        assert!(!source.contains("2 ^ retryCount"));
        assert!(source.contains("var timeout = Math.random() * Math.pow(2, retryCount) * 500;"));
        // The cap: at most 2^7 * 500 ms.
        assert!(source.contains("ensureEventSourceOnElement(elt, Math.min(7, retryCount + 1));"));
        assert!(source.contains(
            "source.onopen = function(evt) {\n\t\t\tretryCount = 0;\n\t\t\tapi.triggerEvent(elt, \"htmx:sseOpen\""
        ));
    }

    #[test]
    fn the_sse_extension_is_the_htmx_1_x_build_that_registers_sse() {
        let source = std::str::from_utf8(HTMX_SSE).expect("UTF-8 script");
        assert!(source.contains("htmx.defineExtension(\"sse\""));
        assert!(source.contains("sse-connect"));
        assert!(source.contains("sse-swap"));
        // Same-origin script without eval-style code, so `script-src 'self'`
        // is enough.
        assert!(!source.contains("eval("));
        assert!(!source.contains("new Function"));
    }

    #[test]
    fn written_files_are_the_vendored_bytes() {
        let root = tempfile::tempdir().expect("temporary static directory");
        write_static(root.path()).expect("write vendored assets");
        for (name, bytes) in STATIC_FILES {
            assert_eq!(fs::read(root.path().join(name)).expect(name), bytes);
        }
    }
}
