//! Explicit rotation keys shared by encrypted messaging storage adapters.
use crate::{MessagingError, Result};
use aes_gcm::{Aes256Gcm, KeyInit};
use std::fmt;
use zeroize::Zeroizing;
const MAX_KEY_ID_BYTES: usize = 64;
const MAX_KEYS: usize = 8;

/// One 256-bit key retained for encrypted durable message storage.
///
/// Load the bytes from a secret manager or CSPRNG. Passwords are not keys and
/// require a suitable KDF first. The cipher zeroizes retained key material on
/// drop; the caller remains responsible for earlier copies.
pub struct MessagingStorageKey {
    key_id: String,
    pub(crate) cipher: Aes256Gcm,
}

impl MessagingStorageKey {
    /// Constructs a storage key from an explicit rotation ID and exactly 32 bytes.
    pub fn try_new(key_id: impl Into<String>, key: impl AsRef<[u8]>) -> Result<Self> {
        let key_id = key_id.into();
        if !valid_key_id(&key_id) {
            return Err(invalid_key("key ID must use 1 to 64 portable characters"));
        }
        let key: [u8; 32] = key
            .as_ref()
            .try_into()
            .map_err(|_| invalid_key("key must contain exactly 32 bytes"))?;
        let key = Zeroizing::new(key);
        let cipher = Aes256Gcm::new_from_slice(key.as_ref())
            .map_err(|_| MessagingError::StorageEncryptionFailed)?;
        Ok(Self { key_id, cipher })
    }

    /// Returns the non-secret rotation identifier.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

impl fmt::Debug for MessagingStorageKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MessagingStorageKey")
            .field("key_id", &self.key_id)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

/// Bounded keyring whose first key encrypts new records and whose prior keys decrypt old ones.
pub struct MessagingKeyring {
    keys: Vec<MessagingStorageKey>,
}

impl MessagingKeyring {
    /// Starts a keyring with the primary key used for all new writes.
    pub fn new(primary: MessagingStorageKey) -> Self {
        Self {
            keys: vec![primary],
        }
    }

    /// Adds one prior decryption key for bounded rotation.
    pub fn with_decryption_key(mut self, key: MessagingStorageKey) -> Result<Self> {
        if self.keys.len() >= MAX_KEYS {
            return Err(invalid_key("keyring cannot contain more than 8 keys"));
        }
        if self.keys.iter().any(|stored| stored.key_id == key.key_id) {
            return Err(invalid_key("key IDs must be unique within the keyring"));
        }
        self.keys.push(key);
        Ok(self)
    }

    /// Returns the non-secret primary rotation identifier.
    pub fn primary_key_id(&self) -> &str {
        &self.keys[0].key_id
    }

    pub(crate) fn primary(&self) -> &MessagingStorageKey {
        &self.keys[0]
    }

    pub(crate) fn find(&self, key_id: &str) -> Result<&MessagingStorageKey> {
        self.keys
            .iter()
            .find(|key| key.key_id == key_id)
            .ok_or(MessagingError::StorageKeyUnavailable)
    }
}

impl fmt::Debug for MessagingKeyring {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MessagingKeyring")
            .field("primary_key_id", &self.primary_key_id())
            .field("key_count", &self.keys.len())
            .field("keys", &"[REDACTED]")
            .finish()
    }
}

pub(crate) fn valid_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_KEY_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

const fn invalid_key(reason: &'static str) -> MessagingError {
    MessagingError::Invalid {
        field: "durable SQLite encryption key",
        reason,
    }
}
