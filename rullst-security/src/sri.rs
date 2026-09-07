use crate::telemetry::SecurityStore;
use base64::Engine;
use sha2::{Digest, Sha384};
use std::path::{Path, PathBuf};

/// Maximum asset size accepted by file-backed SRI helpers (64 MiB).
pub const MAX_SRI_ASSET_BYTES: u64 = 64 * 1024 * 1024;

/// File-backed SRI generation failure.
#[derive(Debug, thiserror::Error)]
pub enum SriError {
    #[error("failed to inspect SRI asset `{path}`: {source}")]
    Inspect {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("SRI asset `{path}` exceeds the {MAX_SRI_ASSET_BYTES}-byte limit")]
    TooLarge { path: PathBuf },
    #[error("failed to read SRI asset `{path}`: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Computes the SHA-384 Subresource Integrity (SRI) hash for a byte slice.
pub fn compute_sri_hash(content: &[u8]) -> String {
    let mut hasher = Sha384::new();
    hasher.update(content);
    let digest = hasher.finalize();
    let b64 = base64::engine::general_purpose::STANDARD.encode(digest);
    SecurityStore::global().inc_sri_signed_assets();
    format!("sha384-{}", b64)
}

/// Generates an HTML `<script>` tag with Subresource Integrity (SRI) attributes.
pub fn sri_script_tag(url: &str, content: &[u8]) -> String {
    let hash = compute_sri_hash(content);
    let escaped_url = rullst_core::html::escape_str(url);
    format!(
        "<script src=\"{}\" integrity=\"{}\" crossorigin=\"anonymous\"></script>",
        escaped_url, hash
    )
}

/// Generates an HTML `<link rel="stylesheet">` tag with Subresource Integrity attributes.
pub fn sri_link_tag(url: &str, content: &[u8]) -> String {
    let hash = compute_sri_hash(content);
    let escaped_url = rullst_core::html::escape_str(url);
    format!(
        "<link rel=\"stylesheet\" href=\"{}\" integrity=\"{}\" crossorigin=\"anonymous\" />",
        escaped_url, hash
    )
}

/// Reads a bounded local JavaScript asset and emits its SRI-protected tag.
pub fn sri_script_tag_from_file(url: &str, path: impl AsRef<Path>) -> Result<String, SriError> {
    read_bounded_asset(path.as_ref()).map(|content| sri_script_tag(url, &content))
}

/// Reads a bounded local stylesheet and emits its SRI-protected tag.
pub fn sri_link_tag_from_file(url: &str, path: impl AsRef<Path>) -> Result<String, SriError> {
    read_bounded_asset(path.as_ref()).map(|content| sri_link_tag(url, &content))
}

fn read_bounded_asset(path: &Path) -> Result<Vec<u8>, SriError> {
    let metadata = std::fs::metadata(path).map_err(|source| SriError::Inspect {
        path: path.to_path_buf(),
        source,
    })?;
    if !is_sri_asset_size_allowed(metadata.len()) {
        return Err(SriError::TooLarge {
            path: path.to_path_buf(),
        });
    }
    std::fs::read(path).map_err(|source| SriError::Read {
        path: path.to_path_buf(),
        source,
    })
}

const fn is_sri_asset_size_allowed(size: u64) -> bool {
    size <= MAX_SRI_ASSET_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sri_hash_computation() {
        let content = b"console.log('Hello Rullst Security!');";
        let hash = compute_sri_hash(content);
        assert!(hash.starts_with("sha384-"));
        assert!(hash.len() > 20);
    }

    #[test]
    fn test_sri_script_tag() {
        let content = b"alert(1);";
        let tag = sri_script_tag("/assets/app.js", content);
        assert!(tag.contains("src=\"/assets/app.js\""));
        assert!(tag.contains("integrity=\"sha384-"));
        assert!(tag.contains("crossorigin=\"anonymous\""));
    }

    #[test]
    fn asset_size_policy_accepts_the_exact_ceiling_only() {
        assert!(is_sri_asset_size_allowed(MAX_SRI_ASSET_BYTES));
        assert!(!is_sri_asset_size_allowed(MAX_SRI_ASSET_BYTES + 1));
    }

    #[test]
    fn file_backed_helpers_hash_real_assets_and_escape_urls() {
        let path = std::env::temp_dir().join(format!("rullst-sri-{}.js", rand::random::<u64>()));
        std::fs::write(&path, b"console.log('safe');").expect("temporary asset");
        let tag =
            sri_script_tag_from_file("/asset.js?x=\"unsafe", &path).expect("file-backed SRI tag");
        assert!(tag.contains("integrity=\"sha384-"));
        assert!(tag.contains("src=\"/asset.js?x=&quot;unsafe\""));
        std::fs::remove_file(path).expect("temporary asset cleanup");
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_sri_asset_size_boundary() {
        let size: u64 = kani::any();
        let allowed = is_sri_asset_size_allowed(size);

        assert_eq!(allowed, size <= MAX_SRI_ASSET_BYTES);
    }
}
