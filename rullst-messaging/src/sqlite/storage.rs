//! Authenticated encryption profile for durable SQLite message contents.

mod crypto;

use crate::storage_keys::{MessagingKeyring, MessagingStorageKey, valid_key_id};
use crate::{MessageHeaders, MessagingError, Namespace, Result};
use std::sync::Arc;
use zeroize::{Zeroize, Zeroizing};

use self::crypto::{open, seal};

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
