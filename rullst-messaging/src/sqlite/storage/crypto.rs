//! Binary AES-256-GCM envelope bound to immutable SQLite row metadata.

use super::{MessageBinding, MessagingKeyring, MessagingStorageKey, valid_key_id};
use crate::{MessagingError, Result};
use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, Generate, Nonce, Payload},
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"RLMSGDB1";
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;
const DOMAIN: &[u8] = b"rullst.messaging.sqlite.aes-256-gcm.v1";

pub(super) fn seal(
    key: &MessagingStorageKey,
    binding: MessageBinding<'_>,
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    let nonce = Nonce::<Aes256Gcm>::try_generate()
        .map_err(|_| MessagingError::StorageRandomnessUnavailable)?;
    let aad = authenticated_data(key.key_id(), binding)?;
    let ciphertext = key
        .cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| MessagingError::StorageEncryptionFailed)?;
    let length = MAGIC
        .len()
        .checked_add(1)
        .and_then(|value| value.checked_add(key.key_id().len()))
        .and_then(|value| value.checked_add(NONCE_BYTES))
        .and_then(|value| value.checked_add(ciphertext.len()))
        .ok_or(MessagingError::StorageEncryptionFailed)?;
    let mut envelope = Vec::with_capacity(length);
    envelope.extend_from_slice(MAGIC);
    envelope.push(key.key_id().len() as u8);
    envelope.extend_from_slice(key.key_id().as_bytes());
    envelope.extend_from_slice(&nonce);
    envelope.extend_from_slice(&ciphertext);
    Ok(envelope)
}

pub(super) fn open<'key>(
    keyring: &'key MessagingKeyring,
    binding: MessageBinding<'_>,
    envelope: &[u8],
) -> Result<(&'key str, Zeroizing<Vec<u8>>)> {
    let minimum = MAGIC.len() + 1 + NONCE_BYTES + TAG_BYTES;
    if envelope.len() < minimum || envelope.get(..MAGIC.len()) != Some(MAGIC.as_slice()) {
        return Err(MessagingError::StorageAuthenticationFailed);
    }
    let key_id_length = envelope
        .get(MAGIC.len())
        .copied()
        .map(usize::from)
        .ok_or(MessagingError::StorageAuthenticationFailed)?;
    let key_start = MAGIC.len() + 1;
    let key_end = key_start
        .checked_add(key_id_length)
        .ok_or(MessagingError::StorageAuthenticationFailed)?;
    let key_id = std::str::from_utf8(
        envelope
            .get(key_start..key_end)
            .ok_or(MessagingError::StorageAuthenticationFailed)?,
    )
    .map_err(|_| MessagingError::StorageAuthenticationFailed)?;
    if !valid_key_id(key_id) {
        return Err(MessagingError::StorageAuthenticationFailed);
    }
    let nonce_end = key_end
        .checked_add(NONCE_BYTES)
        .ok_or(MessagingError::StorageAuthenticationFailed)?;
    let nonce = Nonce::<Aes256Gcm>::try_from(
        envelope
            .get(key_end..nonce_end)
            .ok_or(MessagingError::StorageAuthenticationFailed)?,
    )
    .map_err(|_| MessagingError::StorageAuthenticationFailed)?;
    let ciphertext = envelope
        .get(nonce_end..)
        .filter(|bytes| bytes.len() >= TAG_BYTES)
        .ok_or(MessagingError::StorageAuthenticationFailed)?;
    let key = keyring.find(key_id)?;
    let aad = authenticated_data(key_id, binding)?;
    let plaintext = key
        .cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad: &aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| MessagingError::StorageAuthenticationFailed)?;
    Ok((key.key_id(), plaintext))
}

fn authenticated_data(key_id: &str, binding: MessageBinding<'_>) -> Result<Vec<u8>> {
    let mut aad = Vec::with_capacity(256);
    field(&mut aad, DOMAIN)?;
    field(&mut aad, key_id.as_bytes())?;
    field(&mut aad, binding.namespace.as_bytes())?;
    field(&mut aad, binding.topic.as_bytes())?;
    field(&mut aad, &binding.sequence.to_be_bytes())?;
    field(&mut aad, binding.message_id.as_bytes())?;
    field(&mut aad, binding.event_kind.as_bytes())?;
    field(&mut aad, binding.content_type.as_bytes())?;
    field(&mut aad, &binding.published_at_ms.to_be_bytes())?;
    Ok(aad)
}

fn field(destination: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let length = u32::try_from(value.len()).map_err(|_| MessagingError::StorageEncryptionFailed)?;
    destination.extend_from_slice(&length.to_be_bytes());
    destination.extend_from_slice(value);
    Ok(())
}
