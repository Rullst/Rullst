use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand::RngExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(val: &str) -> Self {
        SecretString(val.to_string())
    }

    /// Reveals the real value only when explicitly requested.
    /// In a real implementation, this might take an `AuditLogToken` to log the access.
    pub fn reveal_audited(&self) -> &str {
        &self.0
    }
}

// In standard debug, it should never leak.
impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[ENCRYPTED_SECRET]")
    }
}

pub struct PrivacyReport {
    pub table_name: String,
    pub has_encrypted_data: bool,
    pub encrypted_fields: Vec<&'static str>,
}

pub trait ComplianceModel {
    fn compliance_schema() -> PrivacyReport;
}

pub fn encrypt_aes_gcm(plaintext: &str, key: &str) -> Result<String, String> {
    let key_bytes = key.as_bytes();
    if key_bytes.len() != 32 {
        return Err("RULLST_ENCRYPTION_KEY must be exactly 32 bytes long".to_string());
    }

    let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|e| e.to_string())?;

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill(&mut nonce_bytes);
    let nonce = Nonce::try_from(&nonce_bytes[..]).map_err(|e| e.to_string())?;

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut payload = nonce_bytes.to_vec();
    payload.extend_from_slice(&ciphertext);

    Ok(STANDARD.encode(payload))
}

pub fn decrypt_aes_gcm(encrypted: &str, key: &str) -> Result<String, String> {
    let key_bytes = key.as_bytes();
    if key_bytes.len() != 32 {
        return Err("RULLST_ENCRYPTION_KEY must be exactly 32 bytes long".to_string());
    }

    let payload = STANDARD.decode(encrypted).map_err(|e| e.to_string())?;
    if payload.len() < 12 {
        return Err("Invalid encrypted payload (too short)".to_string());
    }

    let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|e| e.to_string())?;
    let nonce = Nonce::try_from(&payload[..12]).map_err(|e| e.to_string())?;
    let ciphertext = &payload[12..];

    let plaintext = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| e.to_string())?;

    String::from_utf8(plaintext).map_err(|e| e.to_string())
}

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
impl<'r> sqlx::Decode<'r, sqlx::Any> for SecretString {
    fn decode(
        value: sqlx::any::AnyValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let text = <String as sqlx::Decode<sqlx::Any>>::decode(value)?;
        let encryption_key = std::env::var("RULLST_ENCRYPTION_KEY")
            .map_err(|_| "RULLST_ENCRYPTION_KEY is not set in environment")?;

        let decrypted = decrypt_aes_gcm(&text, &encryption_key)?;
        Ok(SecretString(decrypted))
    }
}

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
impl<'q> sqlx::Encode<'q, sqlx::Any> for SecretString {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Any as sqlx::database::Database>::ArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let encryption_key = std::env::var("RULLST_ENCRYPTION_KEY")
            .map_err(|_| "RULLST_ENCRYPTION_KEY is not set in environment")?;

        let encrypted = encrypt_aes_gcm(&self.0, &encryption_key)?;
        <String as sqlx::Encode<sqlx::Any>>::encode(encrypted, buf)
    }
}

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
impl sqlx::Type<sqlx::Any> for SecretString {
    fn type_info() -> sqlx::any::AnyTypeInfo {
        <String as sqlx::Type<sqlx::Any>>::type_info()
    }
}

// Support for strictly typed databases in Rullst
#[cfg_attr(test, mutants::skip)]
#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
impl<'r> sqlx::Decode<'r, crate::database::RullstDatabase> for SecretString {
    fn decode(
        value: <crate::database::RullstDatabase as sqlx::database::Database>::ValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let text = <String as sqlx::Decode<crate::database::RullstDatabase>>::decode(value)?;
        let encryption_key = std::env::var("RULLST_ENCRYPTION_KEY")
            .map_err(|_| "RULLST_ENCRYPTION_KEY is not set in environment")?;

        let decrypted = decrypt_aes_gcm(&text, &encryption_key)?;
        Ok(SecretString(decrypted))
    }
}

#[cfg_attr(test, mutants::skip)]
#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
impl<'q> sqlx::Encode<'q, crate::database::RullstDatabase> for SecretString {
    fn encode_by_ref(
        &self,
        buf: &mut <crate::database::RullstDatabase as sqlx::database::Database>::ArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let encryption_key = std::env::var("RULLST_ENCRYPTION_KEY")
            .map_err(|_| "RULLST_ENCRYPTION_KEY is not set in environment")?;

        let encrypted = encrypt_aes_gcm(&self.0, &encryption_key)?;
        <String as sqlx::Encode<crate::database::RullstDatabase>>::encode(encrypted, buf)
    }
}

#[cfg_attr(test, mutants::skip)]
#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
impl sqlx::Type<crate::database::RullstDatabase> for SecretString {
    fn type_info() -> <crate::database::RullstDatabase as sqlx::database::Database>::TypeInfo {
        <String as sqlx::Type<crate::database::RullstDatabase>>::type_info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_string_encryption() {
        let key = "01234567890123456789012345678901"; // 32 bytes
        let plaintext = "Sensitive Data 123";

        let encrypted = encrypt_aes_gcm(plaintext, key).unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = decrypt_aes_gcm(&encrypted, key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_secret_string_debug() {
        let secret = SecretString::new("my-cpf-123");
        let debug_str = format!("{:?}", secret);
        assert_eq!(debug_str, "[ENCRYPTED_SECRET]");
    }

    #[test]
    fn test_decrypt_aes_gcm_invalid_length() {
        let key = "01234567890123456789012345678901";

        let short_payload = STANDARD.encode([0u8; 11]);
        let result = decrypt_aes_gcm(&short_payload, key);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid encrypted payload (too short)");

        let exactly_12_payload = STANDARD.encode([0u8; 12]);
        let result = decrypt_aes_gcm(&exactly_12_payload, key);
        assert!(result.is_err());
        assert_ne!(result.unwrap_err(), "Invalid encrypted payload (too short)");
    }

    #[test]
    fn test_secret_string_reveal_audited() {
        let secret = SecretString::new("my-secret-data");
        assert_eq!(secret.reveal_audited(), "my-secret-data");
    }
}
