//! Authenticated encryption profile for durable SQLite message contents.

mod crypto;

use crate::{MessageHeaders, MessagingError, Namespace, Result};
use aes_gcm::{Aes256Gcm, KeyInit};
use std::fmt;
use std::sync::Arc;
use zeroize::{Zeroize, Zeroizing};

use self::crypto::{open, seal};

const MAX_KEY_ID_BYTES: usize = 64;
const MAX_KEYS: usize = 8;
const PLAINTEXT_PROFILE: &str = "plaintext-v1";
const ENCRYPTED_PROFILE: &str = "aes-256-gcm-v1";
const ENCRYPTED_MARKER_PREFIX: &str = "rullst.messaging.encrypted.v1:";
const PROBE_PLAINTEXT: &[u8] = b"rullst.messaging.storage.probe.v1";

#[derive(Clone, Copy)]
pub(super) struct MessageBinding<'value> {
    namespace: &'value str,
    topic: &'value str,
    sequence: i64,
    message_id: &'value str,
    event_kind: &'value str,
    content_type: &'value str,
    published_at_ms: i64,
}

impl<'value> MessageBinding<'value> {
    pub(super) fn message(
        namespace: &'value Namespace,
        topic: &'value str,
        sequence: i64,
        message_id: &'value str,
        event_kind: &'value str,
        content_type: &'value str,
        published_at_ms: i64,
    ) -> Self {
        Self {
            namespace: namespace.as_str(),
            topic,
            sequence,
            message_id,
            event_kind,
            content_type,
            published_at_ms,
        }
    }

    fn probe(namespace: &'value Namespace) -> Self {
        Self {
            namespace: namespace.as_str(),
            topic: "storage-profile-probe",
            sequence: 0,
            message_id: "probe",
            event_kind: "storage.profile",
            content_type: "application/octet-stream",
            published_at_ms: 0,
        }
    }
}

/// One 256-bit key retained for encrypted SQLite message storage.
///
/// Load the bytes from a secret manager or CSPRNG. Passwords are not keys and
/// require a suitable KDF first. The cipher zeroizes retained key material on
/// drop; the caller remains responsible for earlier copies.
pub struct MessagingStorageKey {
    key_id: String,
    cipher: Aes256Gcm,
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

    fn primary(&self) -> &MessagingStorageKey {
        &self.keys[0]
    }

    fn find(&self, key_id: &str) -> Result<&MessagingStorageKey> {
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

#[derive(Clone)]
pub(super) enum StorageProfile {
    Plaintext,
    Encrypted(Arc<MessagingKeyring>),
}

impl StorageProfile {
    pub(super) const fn plaintext() -> Self {
        Self::Plaintext
    }

    pub(super) fn encrypted(keyring: MessagingKeyring) -> Self {
        Self::Encrypted(Arc::new(keyring))
    }

    pub(super) const fn profile_name(&self) -> &'static str {
        match self {
            Self::Plaintext => PLAINTEXT_PROFILE,
            Self::Encrypted(_) => ENCRYPTED_PROFILE,
        }
    }

    pub(super) fn primary_key_id(&self) -> Option<&str> {
        match self {
            Self::Plaintext => None,
            Self::Encrypted(keyring) => Some(keyring.primary_key_id()),
        }
    }

    pub(super) fn seal_probe(&self, namespace: &Namespace) -> Result<Vec<u8>> {
        match self {
            Self::Plaintext => Ok(Vec::new()),
            Self::Encrypted(keyring) => seal(
                keyring.primary(),
                MessageBinding::probe(namespace),
                PROBE_PLAINTEXT,
            ),
        }
    }

    pub(super) fn open_probe(&self, namespace: &Namespace, probe: &[u8]) -> Result<String> {
        match self {
            Self::Plaintext if probe.is_empty() => Ok(String::new()),
            Self::Plaintext => Err(MessagingError::ConfigurationConflict),
            Self::Encrypted(keyring) => {
                let (key_id, plaintext) = open(keyring, MessageBinding::probe(namespace), probe)?;
                if plaintext.as_slice() != PROBE_PLAINTEXT {
                    return Err(MessagingError::StorageAuthenticationFailed);
                }
                Ok(key_id.to_string())
            }
        }
    }

    pub(super) fn ensure_key_available(&self, marker: &str) -> Result<()> {
        let Self::Encrypted(keyring) = self else {
            return Err(MessagingError::ConfigurationConflict);
        };
        let key_id = marker_key_id(marker)?;
        keyring.find(key_id).map(|_| ())
    }

    pub(super) fn encode_message(
        &self,
        binding: MessageBinding<'_>,
        headers: &MessageHeaders,
        payload: &[u8],
    ) -> Result<(String, Vec<u8>)> {
        match self {
            Self::Plaintext => Ok((
                serde_json::to_string(headers).map_err(|_| MessagingError::InternalState {
                    context: "durable header serialization",
                })?,
                payload.to_vec(),
            )),
            Self::Encrypted(keyring) => {
                let header_bytes = Zeroizing::new(serde_json::to_vec(headers).map_err(|_| {
                    MessagingError::InternalState {
                        context: "durable header serialization",
                    }
                })?);
                let header_length = u32::try_from(header_bytes.len()).map_err(|_| {
                    MessagingError::InternalState {
                        context: "durable header length",
                    }
                })?;
                let mut plaintext = Zeroizing::new(Vec::with_capacity(
                    4usize
                        .checked_add(header_bytes.len())
                        .and_then(|length| length.checked_add(payload.len()))
                        .ok_or(MessagingError::StorageEncryptionFailed)?,
                ));
                plaintext.extend_from_slice(&header_length.to_be_bytes());
                plaintext.extend_from_slice(&header_bytes);
                plaintext.extend_from_slice(payload);
                let encrypted = seal(keyring.primary(), binding, plaintext.as_slice())?;
                Ok((
                    format!("{ENCRYPTED_MARKER_PREFIX}{}", keyring.primary_key_id()),
                    encrypted,
                ))
            }
        }
    }

    pub(super) fn decode_message(
        &self,
        binding: MessageBinding<'_>,
        headers_value: String,
        stored_payload: Vec<u8>,
        max_payload_bytes: usize,
    ) -> Result<(MessageHeaders, Vec<u8>)> {
        let (header_bytes, payload) = match self {
            Self::Plaintext => (Zeroizing::new(headers_value.into_bytes()), stored_payload),
            Self::Encrypted(keyring) => {
                let marker_key = marker_key_id(&headers_value)?;
                let (envelope_key, mut plaintext) = open(keyring, binding, &stored_payload)?;
                if marker_key != envelope_key || plaintext.len() < 4 {
                    return Err(MessagingError::StorageAuthenticationFailed);
                }
                let length_bytes: [u8; 4] = plaintext
                    .get(..4)
                    .ok_or(MessagingError::StorageAuthenticationFailed)?
                    .try_into()
                    .map_err(|_| MessagingError::StorageAuthenticationFailed)?;
                let header_length = usize::try_from(u32::from_be_bytes(length_bytes))
                    .map_err(|_| MessagingError::StorageAuthenticationFailed)?;
                let header_end = 4usize
                    .checked_add(header_length)
                    .ok_or(MessagingError::StorageAuthenticationFailed)?;
                let header_bytes = Zeroizing::new(
                    plaintext
                        .get(4..header_end)
                        .ok_or(MessagingError::StorageAuthenticationFailed)?
                        .to_vec(),
                );
                let payload = plaintext
                    .get(header_end..)
                    .ok_or(MessagingError::StorageAuthenticationFailed)?
                    .to_vec();
                plaintext.zeroize();
                (header_bytes, payload)
            }
        };
        if payload.len() > max_payload_bytes {
            return Err(MessagingError::CorruptStorage {
                context: "message bounds",
            });
        }
        let header_map = serde_json::from_slice(header_bytes.as_slice()).map_err(|_| {
            MessagingError::CorruptStorage {
                context: "message header encoding",
            }
        })?;
        Ok((MessageHeaders::from_stored(header_map)?, payload))
    }
}

fn marker_key_id(marker: &str) -> Result<&str> {
    let key_id = marker
        .strip_prefix(ENCRYPTED_MARKER_PREFIX)
        .filter(|key_id| valid_key_id(key_id))
        .ok_or(MessagingError::StorageAuthenticationFailed)?;
    Ok(key_id)
}

fn valid_key_id(value: &str) -> bool {
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
