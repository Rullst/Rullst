//! The stylesheet, script and flags under `/assets/`, embedded at build time from `docs/`.

use sha2::{Digest, Sha256};

const CSS: &[u8] = include_bytes!("../../docs/site.css");
const JS: &[u8] = include_bytes!("../../docs/site.js");
const FLAG_US: &[u8] = include_bytes!("../../docs/flags/us.svg");
const FLAG_BR: &[u8] = include_bytes!("../../docs/flags/br.svg");

/// Published path of each asset and its bytes.
pub const FILES: [(&str, &[u8]); 4] = [
    ("assets/site.css", CSS),
    ("assets/site.js", JS),
    ("assets/flags/us.svg", FLAG_US),
    ("assets/flags/br.svg", FLAG_BR),
];

/// The content-versioned references the pages use (`/assets/site.css?v=<16 hex digits>`).
pub struct Assets {
    pub css: String,
    pub js: String,
}

/// The first 16 hexadecimal digits of the SHA-256 of `bytes`: the asset's content version.
pub fn version(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl Assets {
    pub fn new() -> Self {
        Assets {
            css: format!("/assets/site.css?v={}", version(CSS)),
            js: format!("/assets/site.js?v={}", version(JS)),
        }
    }

    /// The asset files to write next to the pages.
    pub fn files(&self) -> impl Iterator<Item = (&'static str, &'static [u8])> {
        FILES.into_iter()
    }

    /// Whether the site publishes this path (without a leading slash or query).
    pub fn publishes(path: &str) -> bool {
        FILES.iter().any(|(published, _)| *published == path)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_sixteen_hex_digits_of_sha256() {
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(super::version(b"abc"), "ba7816bf8f01cfea");
    }
}
