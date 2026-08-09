use crate::telemetry::SecurityStore;
use base64::Engine;
use sha2::{Digest, Sha384};

/// Computes the SHA-384 Subresource Integrity (SRI) hash for a byte slice.
pub fn compute_sri_hash(content: &[u8]) -> String {
    let mut hasher = Sha384::new();
    hasher.update(content);
    let digest = hasher.finalize();
    let b64 = base64::engine::general_purpose::STANDARD.encode(digest);
    SecurityStore::global().inc_sri_signed_assets();
    format!("sha384-{}", b64)
}

/// Generates an HTML <script> tag with Subresource Integrity (SRI) attributes.
pub fn sri_script_tag(url: &str, content: &[u8]) -> String {
    let hash = compute_sri_hash(content);
    let escaped_url = rullst_core::html::escape_str(url);
    format!(
        "<script src=\"{}\" integrity=\"{}\" crossorigin=\"anonymous\"></script>",
        escaped_url, hash
    )
}

/// Generates an HTML <link rel="stylesheet"> tag with Subresource Integrity (SRI) attributes.
pub fn sri_link_tag(url: &str, content: &[u8]) -> String {
    let hash = compute_sri_hash(content);
    let escaped_url = rullst_core::html::escape_str(url);
    format!(
        "<link rel=\"stylesheet\" href=\"{}\" integrity=\"{}\" crossorigin=\"anonymous\" />",
        escaped_url, hash
    )
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
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_compute_sri_hash_format() {
        let bytes: [u8; 4] = kani::any();
        let hash = compute_sri_hash(&bytes);
        assert!(hash.starts_with("sha384-"));
    }
}
