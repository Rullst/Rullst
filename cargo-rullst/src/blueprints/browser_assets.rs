//! Browser assets vendored into the `static/` directory of HTML scaffolds,
//! served from the application's own origin so pages need no CDN and run
//! under the strict `script-src 'self'` policy. `assets/README.md` records
//! their provenance.

use std::fs;
use std::path::Path;

/// htmx 2.0.11, `package/dist/htmx.min.js` of the npm tarball
/// <https://registry.npmjs.org/htmx.org/-/htmx.org-2.0.11.tgz> (registry
/// integrity `sha512-Thx/WtpeOQqSrqBCw/A1cwGJGg4UrVa3+sW0GmrM3p4gJgO89ecH4qtbnyzDDWFvBTqjnIMCgELTNt636dtamA==`)
/// plus a terminal LF.
/// SHA-256 `7f14135fe8ee3ed5f1a85fea21ba4dfba8236d697c4f95ab575d1c9091bb06ca`.
pub(crate) const HTMX: &[u8] = include_bytes!("assets/htmx-2.0.11.min.js");

/// The htmx 2 Server-Sent Events extension 2.2.4, `package/dist/sse.js` of
/// the npm tarball
/// <https://registry.npmjs.org/htmx-ext-sse/-/htmx-ext-sse-2.2.4.tgz>
/// (registry integrity `sha512-LJmxVhykyflBWgh5PvbRidcyuqMHlgfajmmzumvKctv9puvsufeH6OaejSMZTNTnEI8O2wXXn4ZZtBdpEMqmEQ==`),
/// unmodified: its reconnect delay doubles up to 64 seconds and resets when a
/// connection opens, so the patch Rullst carried on the htmx 1.9.12
/// extension (whose delay used the bitwise XOR `2 ^ retryCount`) is gone.
/// SHA-256 `3b5992a541619babefc4c169505af474df5c3039da51e59b96ccf9241ecd61d2`.
pub(crate) const HTMX_SSE: &[u8] = include_bytes!("assets/htmx-ext-sse-2.2.4.js");

/// The Zero-Clause BSD license of htmx.
pub(crate) const HTMX_LICENSE: &[u8] = include_bytes!("assets/HTMX-LICENSE");

/// The Zero-Clause BSD license of the SSE extension (its own repository).
pub(crate) const HTMX_SSE_LICENSE: &[u8] = include_bytes!("assets/HTMX-EXT-SSE-LICENSE");

/// Every vendored file, by its name under `static/`.
pub(crate) const STATIC_FILES: [(&str, &[u8]); 4] = [
    ("htmx-2.0.11.min.js", HTMX),
    ("htmx-ext-sse-2.2.4.js", HTMX_SSE),
    ("HTMX-LICENSE", HTMX_LICENSE),
    ("HTMX-EXT-SSE-LICENSE", HTMX_SSE_LICENSE),
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
    const HTMX_SHA256: &str = "7f14135fe8ee3ed5f1a85fea21ba4dfba8236d697c4f95ab575d1c9091bb06ca";
    const HTMX_SSE_SHA256: &str =
        "3b5992a541619babefc4c169505af474df5c3039da51e59b96ccf9241ecd61d2";
    /// Upstream `dist/htmx.min.js`, before the terminal LF.
    const UPSTREAM_HTMX_SHA256: &str =
        "d6fdc75f204e6bdefa99b69bf1e6d4ac69b8a364f77929f45c13476b4000f717";

    fn sha256(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    #[test]
    fn vendored_files_keep_their_recorded_digests() {
        assert_eq!(sha256(HTMX), HTMX_SHA256);
        assert_eq!(sha256(HTMX_SSE), HTMX_SSE_SHA256);
        // The upstream bytes plus one LF.
        assert_eq!(HTMX.last(), Some(&b'\n'));
        assert_eq!(sha256(&HTMX[..HTMX.len() - 1]), UPSTREAM_HTMX_SHA256);
        let readme = include_str!("assets/README.md");
        let source = include_str!("browser_assets.rs");
        for digest in [HTMX_SHA256, HTMX_SSE_SHA256] {
            assert!(readme.contains(digest), "{digest}");
            // The item documentation and the constant in this test.
            assert_eq!(source.matches(digest).count(), 2, "{digest}");
        }
        assert!(readme.contains(UPSTREAM_HTMX_SHA256));
    }

    #[test]
    fn htmx_is_the_2_0_11_release() {
        let source = std::str::from_utf8(HTMX).expect("UTF-8 script");
        assert!(source.starts_with("var htmx=function(){\"use strict\";"));
        assert!(source.contains("version:\"2.0.11\""));
        assert!(!source.contains("sourceMappingURL"));
    }

    #[test]
    fn the_sse_extension_backs_off_exponentially_and_resets_on_open() {
        let source = std::str::from_utf8(HTMX_SSE).expect("UTF-8 script");
        // The bitwise XOR of the htmx 1.9.12 extension is gone upstream.
        assert!(!source.contains("2 ^ retryCount"));
        assert!(source.contains("retryCount = Math.max(Math.min(retryCount * 2, 128), 1)"));
        assert!(source.contains("var timeout = retryCount * 500"));
        assert!(source.contains("        retryCount = 0\n"));
    }

    #[test]
    fn the_sse_extension_is_the_htmx_2_build_that_registers_sse() {
        let source = std::str::from_utf8(HTMX_SSE).expect("UTF-8 script");
        assert!(source.contains("htmx.defineExtension('sse'"));
        assert!(source.contains("sse-connect"));
        assert!(source.contains("sse-swap"));
        // htmx 2 dropped the `hx-sse` attribute the 1.x extension also read.
        assert!(!source.contains("hx-sse"));
        // Same-origin script without eval-style code, so `script-src 'self'`
        // is enough.
        assert!(!source.contains("eval("));
        assert!(!source.contains("Function("));
    }

    #[test]
    fn both_licenses_are_zero_clause_bsd() {
        for license in [HTMX_LICENSE, HTMX_SSE_LICENSE] {
            let text = std::str::from_utf8(license).expect("UTF-8 license");
            assert!(text.contains("Zero"), "{text}");
            assert!(text.contains("Permission to use, copy, modify, and/or distribute"));
        }
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
