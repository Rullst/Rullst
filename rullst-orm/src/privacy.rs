use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, Generate, KeyInit, Nonce, Payload},
};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use serde::{Deserialize, Serialize};

const ENVELOPE_PREFIX: &str = "RULLST";
const ENVELOPE_VERSION: &str = "v2";
const DEFAULT_KEY_ID: &str = "default";
const KEY_LENGTH: usize = 32;
const NONCE_LENGTH: usize = 12;
const TAG_LENGTH: usize = 16;
const MAX_KEY_ID_LENGTH: usize = 128;
const AAD_DOMAIN: &[u8] = b"rullst-orm:field-encryption:aes-256-gcm";
const KEY_ENV: &str = "RULLST_ENCRYPTION_KEY";
const KEY_ID_ENV: &str = "RULLST_ENCRYPTION_KEY_ID";
const KEYRING_ENV: &str = "RULLST_ENCRYPTION_KEYRING";

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(val: impl Into<String>) -> Self {
        SecretString(val.into())
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

/// Strongly-typed error domain for Rullst ORM privacy and column encryption.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrivacyError {
    /// The encryption key is missing or not exactly 32 bytes long.
    #[error("RULLST_ENCRYPTION_KEY must be exactly 32 bytes long")]
    InvalidKeyLength,

    /// A prefixed base64 or hexadecimal key could not be decoded.
    #[error("RULLST_ENCRYPTION_KEY has invalid {0} encoding")]
    InvalidKeyEncoding(&'static str),

    /// A key identifier was empty, too long, or contained an envelope delimiter.
    #[error("Invalid field-encryption key identifier")]
    InvalidKeyId,

    /// A versioned encrypted value did not contain the required fields.
    #[error("Invalid field-encryption envelope")]
    InvalidEnvelope,

    /// The encrypted value belongs to an unknown future format.
    #[error("Unsupported field-encryption envelope version: {0}")]
    UnsupportedEnvelopeVersion(String),

    /// The configured keyring does not contain the key selected by the envelope.
    #[error("Field-encryption key `{0}` was not found in the configured keyring")]
    KeyNotFound(String),

    /// The operating system could not provide a cryptographically secure nonce.
    #[error("Operating-system randomness is unavailable")]
    RandomnessUnavailable,

    /// AES-GCM encryption failed.
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    /// AES-GCM decryption failed.
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// The payload is shorter than the 12-byte nonce requirement.
    #[error("Invalid encrypted payload (too short)")]
    PayloadTooShort,

    /// Base64 decoding failed.
    #[error("Base64 decode error: {0}")]
    Base64Error(String),

    /// Decrypted payload is not valid UTF-8.
    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(String),

    /// Missing environment variable key.
    #[error("Environment variable error: {0}")]
    EnvError(String),
}

fn validate_key(key: &[u8]) -> Result<(), PrivacyError> {
    if key.len() == KEY_LENGTH {
        Ok(())
    } else {
        Err(PrivacyError::InvalidKeyLength)
    }
}

fn validate_key_id(key_id: &str) -> Result<(), PrivacyError> {
    if key_id.is_empty()
        || key_id.len() > MAX_KEY_ID_LENGTH
        || !key_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        Err(PrivacyError::InvalidKeyId)
    } else {
        Ok(())
    }
}

fn authenticated_data(key_id: &str, context: &[u8]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(AAD_DOMAIN.len() + key_id.len() + context.len() + 4);
    aad.extend_from_slice(AAD_DOMAIN);
    aad.push(0);
    aad.extend_from_slice(ENVELOPE_VERSION.as_bytes());
    aad.push(0);
    aad.extend_from_slice(key_id.as_bytes());
    aad.push(0);
    aad.extend_from_slice(context);
    aad
}

fn model_context(table: &str, column: &str) -> Vec<u8> {
    let mut context = Vec::with_capacity(table.len() + column.len() + 1);
    context.extend_from_slice(table.as_bytes());
    context.push(0);
    context.extend_from_slice(column.as_bytes());
    context
}

fn encrypt_with_context(
    plaintext: &str,
    key: &[u8],
    key_id: &str,
    context: &[u8],
) -> Result<String, PrivacyError> {
    validate_key(key)?;
    validate_key_id(key_id)?;
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|error| PrivacyError::EncryptionFailed(error.to_string()))?;
    let nonce =
        Nonce::<Aes256Gcm>::try_generate().map_err(|_| PrivacyError::RandomnessUnavailable)?;
    let aad = authenticated_data(key_id, context);
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext.as_bytes(),
                aad: &aad,
            },
        )
        .map_err(|error| PrivacyError::EncryptionFailed(error.to_string()))?;

    Ok(format!(
        "{ENVELOPE_PREFIX}:{ENVELOPE_VERSION}:{key_id}:{}:{}",
        URL_SAFE_NO_PAD.encode(nonce),
        URL_SAFE_NO_PAD.encode(ciphertext)
    ))
}

struct ParsedEnvelope<'a> {
    key_id: &'a str,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

fn parse_envelope(encrypted: &str) -> Result<ParsedEnvelope<'_>, PrivacyError> {
    let mut fields = encrypted.split(':');
    if fields.next() != Some(ENVELOPE_PREFIX) {
        return Err(PrivacyError::InvalidEnvelope);
    }
    let version = fields.next().ok_or(PrivacyError::InvalidEnvelope)?;
    if version != ENVELOPE_VERSION {
        return Err(PrivacyError::UnsupportedEnvelopeVersion(
            version.to_string(),
        ));
    }
    let key_id = fields.next().ok_or(PrivacyError::InvalidEnvelope)?;
    validate_key_id(key_id)?;
    let encoded_nonce = fields.next().ok_or(PrivacyError::InvalidEnvelope)?;
    let encoded_ciphertext = fields.next().ok_or(PrivacyError::InvalidEnvelope)?;
    if fields.next().is_some() {
        return Err(PrivacyError::InvalidEnvelope);
    }

    let nonce = URL_SAFE_NO_PAD
        .decode(encoded_nonce)
        .map_err(|error| PrivacyError::Base64Error(error.to_string()))?;
    if nonce.len() != NONCE_LENGTH {
        return Err(PrivacyError::PayloadTooShort);
    }
    let ciphertext = URL_SAFE_NO_PAD
        .decode(encoded_ciphertext)
        .map_err(|error| PrivacyError::Base64Error(error.to_string()))?;
    if ciphertext.len() < TAG_LENGTH {
        return Err(PrivacyError::PayloadTooShort);
    }

    Ok(ParsedEnvelope {
        key_id,
        nonce,
        ciphertext,
    })
}

fn decrypt_envelope(
    envelope: &ParsedEnvelope<'_>,
    key: &[u8],
    context: &[u8],
) -> Result<String, PrivacyError> {
    validate_key(key)?;
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|error| PrivacyError::DecryptionFailed(error.to_string()))?;
    let nonce = Nonce::<Aes256Gcm>::try_from(envelope.nonce.as_slice())
        .map_err(|error| PrivacyError::DecryptionFailed(error.to_string()))?;
    let aad = authenticated_data(envelope.key_id, context);
    let plaintext = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: &envelope.ciphertext,
                aad: &aad,
            },
        )
        .map_err(|error| PrivacyError::DecryptionFailed(error.to_string()))?;
    String::from_utf8(plaintext).map_err(|error| PrivacyError::Utf8Error(error.to_string()))
}

fn environment_variable(name: &str) -> Result<String, PrivacyError> {
    std::env::var(name).map_err(|error| PrivacyError::EnvError(format!("{name}: {error}")))
}

fn current_key_id() -> Result<String, PrivacyError> {
    match std::env::var(KEY_ID_ENV) {
        Ok(key_id) => {
            validate_key_id(&key_id)?;
            Ok(key_id)
        }
        Err(std::env::VarError::NotPresent) => Ok(DEFAULT_KEY_ID.to_string()),
        Err(error) => Err(PrivacyError::EnvError(format!("{KEY_ID_ENV}: {error}"))),
    }
}

fn decode_configured_key(value: &str) -> Result<Vec<u8>, PrivacyError> {
    let key = if let Some(encoded) = value.strip_prefix("base64:") {
        STANDARD
            .decode(encoded)
            .map_err(|_| PrivacyError::InvalidKeyEncoding("base64"))?
    } else if let Some(encoded) = value.strip_prefix("hex:") {
        let (pairs, remainder) = encoded.as_bytes().as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(PrivacyError::InvalidKeyEncoding("hex"));
        }
        pairs
            .iter()
            .map(|pair| {
                let pair = std::str::from_utf8(pair)
                    .map_err(|_| PrivacyError::InvalidKeyEncoding("hex"))?;
                u8::from_str_radix(pair, 16).map_err(|_| PrivacyError::InvalidKeyEncoding("hex"))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        value.as_bytes().to_vec()
    };
    validate_key(&key)?;
    Ok(key)
}

fn current_key() -> Result<Vec<u8>, PrivacyError> {
    decode_configured_key(&environment_variable(KEY_ENV)?)
}

fn configured_key(key_id: &str) -> Result<Vec<u8>, PrivacyError> {
    if current_key_id()? == key_id {
        return current_key();
    }

    let keyring = environment_variable(KEYRING_ENV)?;
    let values: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&keyring)
        .map_err(|error| {
            PrivacyError::EnvError(format!(
                "{KEYRING_ENV} must be a JSON object of key IDs to 32-byte keys: {error}"
            ))
        })?;
    let key = values
        .get(key_id)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| PrivacyError::KeyNotFound(key_id.to_string()))?
        .to_string();
    decode_configured_key(&key)
}

/// Encrypts an ORM model field using the configured current key and an
/// authenticated table/column context.
pub fn encrypt_model_field(
    plaintext: &str,
    table: &str,
    column: &str,
) -> Result<String, PrivacyError> {
    let key_id = current_key_id()?;
    let key = current_key()?;
    encrypt_with_context(plaintext, &key, &key_id, &model_context(table, column))
}

/// Decrypts an ORM model field and selects its key from the configured current
/// key or `RULLST_ENCRYPTION_KEYRING` JSON object.
pub fn decrypt_model_field(
    encrypted: &str,
    table: &str,
    column: &str,
) -> Result<String, PrivacyError> {
    let envelope = parse_envelope(encrypted)?;
    let key = configured_key(envelope.key_id)?;
    decrypt_envelope(&envelope, &key, &model_context(table, column))
}

pub fn encrypt_aes_gcm(plaintext: &str, key: &str) -> Result<String, PrivacyError> {
    encrypt_with_context(plaintext, key.as_bytes(), DEFAULT_KEY_ID, b"")
}

pub fn decrypt_aes_gcm(encrypted: &str, key: &str) -> Result<String, PrivacyError> {
    let key_bytes = key.as_bytes();
    validate_key(key_bytes)?;
    if encrypted.starts_with(&format!("{ENVELOPE_PREFIX}:")) {
        let envelope = parse_envelope(encrypted)?;
        return decrypt_envelope(&envelope, key_bytes, b"");
    }

    decrypt_legacy_aes_gcm(encrypted, key_bytes)
}

fn decrypt_legacy_aes_gcm(encrypted: &str, key: &[u8]) -> Result<String, PrivacyError> {
    // v12 reads the pre-v12 nonce+ciphertext base64 representation so existing
    // `SecretString` rows can be loaded and rewritten into the versioned format.
    let payload = STANDARD
        .decode(encrypted)
        .map_err(|error| PrivacyError::Base64Error(error.to_string()))?;
    if payload.len() < NONCE_LENGTH + TAG_LENGTH {
        return Err(PrivacyError::PayloadTooShort);
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|error| PrivacyError::DecryptionFailed(error.to_string()))?;
    let nonce = Nonce::<Aes256Gcm>::try_from(&payload[..NONCE_LENGTH])
        .map_err(|error| PrivacyError::DecryptionFailed(error.to_string()))?;
    let ciphertext = &payload[NONCE_LENGTH..];

    let plaintext = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|error| PrivacyError::DecryptionFailed(error.to_string()))?;

    String::from_utf8(plaintext).map_err(|error| PrivacyError::Utf8Error(error.to_string()))
}

fn encrypt_configured_secret(plaintext: &str) -> Result<String, PrivacyError> {
    let key_id = current_key_id()?;
    let key = current_key()?;
    encrypt_with_context(plaintext, &key, &key_id, b"")
}

fn decrypt_configured_secret(encrypted: &str) -> Result<String, PrivacyError> {
    if encrypted.starts_with(&format!("{ENVELOPE_PREFIX}:")) {
        let envelope = parse_envelope(encrypted)?;
        let key = configured_key(envelope.key_id)?;
        decrypt_envelope(&envelope, &key, b"")
    } else {
        decrypt_legacy_aes_gcm(encrypted, &current_key()?)
    }
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
        let decrypted = decrypt_configured_secret(&text)?;
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
        let encrypted = encrypt_configured_secret(&self.0)?;
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
        let decrypted = decrypt_configured_secret(&text)?;
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
        let encrypted = encrypt_configured_secret(&self.0)?;
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
mod tests;
