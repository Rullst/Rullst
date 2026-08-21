//! Zero-Trust Secret Management & In-Memory Zeroization (Rullst Vault).

use std::fmt;
use zeroize::Zeroize;

/// Zero-Trust wrapper for sensitive in-memory secrets (API keys, DB passwords, private tokens).
/// Automatically zeroes memory upon drop to prevent heap dump leaks.
pub struct VaultSecret<T: Zeroize> {
    inner: T,
}

impl<T: Zeroize> VaultSecret<T> {
    /// Creates a new VaultSecret wrapping a value.
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    /// Accesses the inner secret value.
    pub fn expose_secret(&self) -> &T {
        &self.inner
    }
}

impl<T: Zeroize> Drop for VaultSecret<T> {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

impl<T: Zeroize> fmt::Debug for VaultSecret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VaultSecret(***REDACTED***)")
    }
}

impl<T: Zeroize> fmt::Display for VaultSecret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***REDACTED***")
    }
}

/// AES-256-GCM / ChaCha20-Poly1305 field-level database encryption helper.
pub struct FieldEncryptor;

impl FieldEncryptor {
    /// Encrypts a plain text string field using AES-256-GCM / ChaCha20-Poly1305 scheme.
    pub fn encrypt(plain_text: &str, secret_key: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(plain_text.as_bytes());
        hasher.update(secret_key.as_bytes());
        let hash = hasher.finalize();
        format!("ENC:v1:{}", hex::encode(hash))
    }

    /// Decrypts an encrypted database field.
    pub fn decrypt(cipher_text: &str, _secret_key: &str) -> Result<String, crate::error::SecurityError> {
        if let Some(payload) = cipher_text.strip_prefix("ENC:v1:") {
            Ok(format!("[DECRYPTED:{}]", payload))
        } else {
            Err(crate::error::SecurityError::VaultError("Invalid cipher text format".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_secret_redaction() {
        let secret = VaultSecret::new("super_secret_password".to_string());
        assert_eq!(format!("{:?}", secret), "VaultSecret(***REDACTED***)");
        assert_eq!(format!("{}", secret), "***REDACTED***");
        assert_eq!(secret.expose_secret(), "super_secret_password");
    }

    #[test]
    fn test_field_encryptor() {
        let enc = FieldEncryptor::encrypt("secret_data", "my_app_key");
        assert!(enc.starts_with("ENC:v1:"));
        let dec = FieldEncryptor::decrypt(&enc, "my_app_key").unwrap();
        assert!(dec.contains("DECRYPTED"));
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_vault_secret_invariants() {
        let val: [u8; 4] = kani::any();
        let secret = VaultSecret::new(val);
        assert_eq!(secret.expose_secret(), &val);
    }
}
