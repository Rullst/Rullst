use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ring::digest::{SHA256, digest};

use crate::fiscal::models::{FiscalCertificate, FiscalError};

/// Computes a standard SHA-256 base64 digest for an XML element string.
pub fn compute_sha256_digest(content: &str) -> String {
    let hash = digest(&SHA256, content.as_bytes());
    STANDARD.encode(hash.as_ref())
}

/// XMLDSig signing is intentionally unavailable until C14N, PKCS#12 private-key extraction and
/// RSA-SHA256 interoperability are independently validated against the official NFS-e contract.
///
/// This function never fabricates a `<Signature>` block.
pub fn sign_dps_xml(_xml: &str, _certificate: &FiscalCertificate) -> Result<String, FiscalError> {
    Err(FiscalError::Unsupported(
        "NFS-e XMLDSig signing is not implemented; no document was signed".to_string(),
    ))
}
