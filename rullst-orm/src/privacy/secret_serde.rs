//! Serde contract for [`SecretString`]: never plaintext.
//!
//! `Serialize` writes an authenticated `RULLST:v2` envelope under the
//! configured key, so secondary stores such as the Redis query cache hold
//! ciphertext that `Deserialize` turns back into the real value. Generated
//! audit, event and search projections run inside [`with_redacted_secrets`],
//! where the value becomes the fixed `"***"` marker and no key is needed.

use super::{
    ENVELOPE_PREFIX, PrivacyError, SecretString, configured_key, current_key, current_key_id,
    decrypt_envelope, encrypt_with_context, parse_envelope,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cell::Cell;

/// Authenticated-data context for serialized secrets. It differs from the SQL
/// column context, so a serialized value and a stored column value are not
/// interchangeable ciphertexts.
const SERDE_CONTEXT: &[u8] = b"rullst-orm:secret-string:serde";

thread_local! {
    static REDACTION_DEPTH: Cell<usize> = const { Cell::new(0) };
}

struct RedactionScope;

impl RedactionScope {
    fn enter() -> Self {
        REDACTION_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
        Self
    }
}

impl Drop for RedactionScope {
    fn drop(&mut self) {
        REDACTION_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// Runs `serialize` with every [`SecretString`] serialized as `"***"`.
///
/// Generated `to_json()` and search projections use this scope. It applies to
/// the current thread for the duration of the synchronous closure only.
#[doc(hidden)]
pub fn with_redacted_secrets<R>(serialize: impl FnOnce() -> R) -> R {
    let _scope = RedactionScope::enter();
    serialize()
}

fn redacting() -> bool {
    REDACTION_DEPTH.with(|depth| depth.get() > 0)
}

fn encrypt_serialized(plaintext: &str) -> Result<String, PrivacyError> {
    let key_id = current_key_id()?;
    let key = current_key()?;
    encrypt_with_context(plaintext, &key, &key_id, SERDE_CONTEXT)
}

fn decrypt_serialized(encrypted: &str) -> Result<String, PrivacyError> {
    let envelope = parse_envelope(encrypted)?;
    let key = configured_key(envelope.key_id)?;
    decrypt_envelope(&envelope, &key, SERDE_CONTEXT)
}

fn is_envelope(value: &str) -> bool {
    value
        .strip_prefix(ENVELOPE_PREFIX)
        .is_some_and(|rest| rest.starts_with(':'))
}

impl Serialize for SecretString {
    /// Emits an encrypted envelope, or `"***"` inside a redacted projection.
    /// Fails when `RULLST_ENCRYPTION_KEY` is not configured.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if redacting() {
            return serializer.serialize_str(crate::audit::REDACTED_VALUE);
        }
        let envelope = encrypt_serialized(&self.0).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&envelope)
    }
}

impl<'de> Deserialize<'de> for SecretString {
    /// Decrypts an envelope written by `Serialize` (current key or keyring).
    /// Any other string is accepted as plaintext input; a malformed or
    /// unauthenticated envelope is rejected.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if !is_envelope(&value) {
            return Ok(SecretString(value));
        }
        decrypt_serialized(&value)
            .map(SecretString)
            .map_err(serde::de::Error::custom)
    }
}
