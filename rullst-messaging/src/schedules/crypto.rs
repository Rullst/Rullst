use super::*;
use crate::storage_keys::valid_key_id;
use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, Generate, Nonce, Payload},
};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"RLSCHED1";

/// Row identities are server-owned. Each purpose has a distinct AEAD domain.
pub(super) fn seal(
    keys: &MessagingKeyring,
    namespace: &str,
    purpose: &str,
    identity: &str,
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    if plaintext.len() > MAX_CONTENT_BYTES {
        return Err(RecurringError::InvalidInput("encrypted content size"));
    }
    let key = keys.primary();
    let nonce = Nonce::<Aes256Gcm>::try_generate().map_err(|_| RecurringError::Randomness)?;
    let aad = aad(namespace, purpose, identity, key.key_id());
    let cipher = key
        .cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| RecurringError::Encryption)?;
    let mut value = Vec::with_capacity(8 + 1 + key.key_id().len() + 12 + cipher.len());
    value.extend_from_slice(MAGIC);
    value.push(key.key_id().len() as u8);
    value.extend_from_slice(key.key_id().as_bytes());
    value.extend_from_slice(&nonce);
    value.extend_from_slice(&cipher);
    Ok(value)
}

pub(super) fn open(
    keys: &MessagingKeyring,
    namespace: &str,
    purpose: &str,
    identity: &str,
    value: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    if value.len() > MAX_CONTENT_BYTES + 128
        || value.len() < 8 + 1 + 1 + 12 + 16
        || value.get(..8) != Some(MAGIC.as_slice())
    {
        return Err(RecurringError::Encryption);
    }
    let length = usize::from(*value.get(8).ok_or(RecurringError::Encryption)?);
    let end = 9 + length;
    let key_id = std::str::from_utf8(value.get(9..end).ok_or(RecurringError::Encryption)?)
        .map_err(|_| RecurringError::Encryption)?;
    if !valid_key_id(key_id) {
        return Err(RecurringError::Encryption);
    }
    let key = keys.find(key_id).map_err(|_| RecurringError::Encryption)?;
    let nonce =
        Nonce::<Aes256Gcm>::try_from(value.get(end..end + 12).ok_or(RecurringError::Encryption)?)
            .map_err(|_| RecurringError::Encryption)?;
    let cipher = value
        .get(end + 12..)
        .filter(|v| v.len() >= 16)
        .ok_or(RecurringError::Encryption)?;
    let aad = aad(namespace, purpose, identity, key_id);
    key.cipher
        .decrypt(
            &nonce,
            Payload {
                msg: cipher,
                aad: &aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| RecurringError::Encryption)
}
fn aad(namespace: &str, purpose: &str, identity: &str, key_id: &str) -> Vec<u8> {
    let mut value = Vec::new();
    for field in [
        "rullst.messaging.recurring.aes256gcm.v1",
        namespace,
        purpose,
        identity,
        key_id,
    ] {
        value.extend_from_slice(&(field.len() as u64).to_be_bytes());
        value.extend_from_slice(field.as_bytes());
    }
    value
}
pub(super) fn random_token() -> Result<Zeroizing<String>> {
    let mut bytes = Zeroizing::new([0u8; 32]);
    getrandom::fill(bytes.as_mut()).map_err(|_| RecurringError::Randomness)?;
    Ok(Zeroizing::new(hex(bytes.as_ref())))
}
pub(super) fn occurrence_id(namespace: &str, generation: &str, due: i64) -> String {
    let mut digest = Sha256::new();
    for part in [
        b"rullst.recurring.occurrence.v1".as_slice(),
        namespace.as_bytes(),
        generation.as_bytes(),
        &due.to_be_bytes(),
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part);
    }
    hex(&digest.finalize())
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut value, "{byte:02x}");
    }
    value
}

pub(super) fn lease_hash(namespace: &str, id: &str, version: i64, token: &str) -> Vec<u8> {
    let mut digest = Sha256::new();
    for value in [
        b"rullst.recurring.lease.v1".as_slice(),
        namespace.as_bytes(),
        id.as_bytes(),
        &version.to_be_bytes(),
        token.as_bytes(),
    ] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value);
    }
    digest.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MessagingStorageKey;
    fn keys(id: &str, byte: u8) -> MessagingKeyring {
        MessagingKeyring::new(MessagingStorageKey::try_new(id, [byte; 32]).unwrap())
    }
    #[test]
    fn encrypted_content_binds_namespace_purpose_identity_key_and_every_byte() {
        let ring = keys("old", 7);
        let encrypted = seal(&ring, "school", "occurrence", "one", b"private").unwrap();
        assert_eq!(
            open(&ring, "school", "occurrence", "one", &encrypted)
                .unwrap()
                .as_slice(),
            b"private"
        );
        for (namespace, purpose, identity) in [
            ("other", "occurrence", "one"),
            ("school", "definition", "one"),
            ("school", "occurrence", "two"),
        ] {
            assert_eq!(
                open(&ring, namespace, purpose, identity, &encrypted),
                Err(RecurringError::Encryption)
            );
        }
        for index in 0..encrypted.len() {
            let mut changed = encrypted.clone();
            changed[index] ^= 1;
            assert_eq!(
                open(&ring, "school", "occurrence", "one", &changed),
                Err(RecurringError::Encryption)
            );
        }
        for length in 0..encrypted.len() {
            assert_eq!(
                open(&ring, "school", "occurrence", "one", &encrypted[..length]),
                Err(RecurringError::Encryption)
            );
        }
        assert!(open(&keys("old", 8), "school", "occurrence", "one", &encrypted).is_err());
        assert!(open(&keys("new", 7), "school", "occurrence", "one", &encrypted).is_err());
        let rotated = keys("new", 8)
            .with_decryption_key(MessagingStorageKey::try_new("old", [7; 32]).unwrap())
            .unwrap();
        assert!(open(&rotated, "school", "occurrence", "one", &encrypted).is_ok());
        assert!(
            seal(
                &ring,
                "school",
                "occurrence",
                "one",
                &vec![0; MAX_CONTENT_BYTES + 1]
            )
            .is_err()
        );
        assert!(
            open(
                &ring,
                "school",
                "occurrence",
                "one",
                &vec![0; MAX_CONTENT_BYTES + 129]
            )
            .is_err()
        );
        let first = random_token().unwrap();
        let second = random_token().unwrap();
        assert_ne!(first, second);
        assert_eq!(first.len(), 64);
        assert_ne!(occurrence_id("a", "b", 1), occurrence_id("a", "b", 2));
        assert_ne!(occurrence_id("a", "b", 1), occurrence_id("ab", "", 1));
        assert_ne!(
            lease_hash("a", "b", 1, &first),
            lease_hash("a", "b", 2, &first)
        );
    }
}
