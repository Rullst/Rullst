use super::{
    OFFLINE_SNAPSHOT_DOMAIN, OfflineAccountId, OfflineSyncError, OfflineSyncPolicy,
    OfflineSyncState,
};
use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, Generate, KeyInit, Nonce, Payload},
};
use std::fmt;
use zeroize::Zeroizing;

const ENVELOPE_MAGIC: &[u8; 8] = b"RLSOFF01";
const KEY_LENGTH: usize = 32;
const NONCE_LENGTH: usize = 12;
const TAG_LENGTH: usize = 16;
const MAX_KEY_ID_BYTES: usize = 64;
const FIXED_ENVELOPE_BYTES: usize = ENVELOPE_MAGIC.len() + 1 + NONCE_LENGTH + TAG_LENGTH;

/// AES-256-GCM snapshot codec with account and rotation-key AAD binding.
///
/// The owned key is zeroized on drop and omitted from `Debug`. This cannot erase
/// copies made before construction. Load keys from platform secure storage and
/// delete both the snapshot and its key when the account is erased.
pub struct OfflineSnapshotCipher {
    key_id: String,
    key: Zeroizing<[u8; KEY_LENGTH]>,
}

impl OfflineSnapshotCipher {
    /// Creates a codec from a bounded rotation id and exactly 32 key bytes.
    pub fn new(key_id: impl Into<String>, key: impl AsRef<[u8]>) -> Result<Self, OfflineSyncError> {
        let key_id = key_id.into();
        if !valid_key_id(&key_id) {
            return Err(OfflineSyncError::InvalidKeyId);
        }
        let source = key.as_ref();
        let key: [u8; KEY_LENGTH] = source
            .try_into()
            .map_err(|_| OfflineSyncError::InvalidKeyLength)?;
        Ok(Self {
            key_id,
            key: Zeroizing::new(key),
        })
    }

    /// Returns the non-secret rotation identifier carried by new envelopes.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Serializes, bounds, and encrypts one account-bound snapshot.
    pub fn seal(
        &self,
        policy: OfflineSyncPolicy,
        state: &OfflineSyncState,
    ) -> Result<Vec<u8>, OfflineSyncError> {
        let plaintext = Zeroizing::new(state.encode(policy)?);
        let cipher = Aes256Gcm::new_from_slice(self.key.as_ref())
            .map_err(|_| OfflineSyncError::InvalidKeyLength)?;
        let nonce = Nonce::<Aes256Gcm>::try_generate()
            .map_err(|_| OfflineSyncError::RandomnessUnavailable)?;
        let aad = authenticated_data(&self.key_id, state.account_id());
        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext.as_slice(),
                    aad: &aad,
                },
            )
            .map_err(|_| OfflineSyncError::EncryptionFailed)?;
        let encoded_len = ENVELOPE_MAGIC
            .len()
            .checked_add(1)
            .and_then(|length| length.checked_add(self.key_id.len()))
            .and_then(|length| length.checked_add(nonce.len()))
            .and_then(|length| length.checked_add(ciphertext.len()))
            .ok_or(OfflineSyncError::SnapshotTooLarge {
                maximum: policy.max_snapshot_bytes(),
            })?;
        let maximum = maximum_envelope_bytes(policy)?;
        if encoded_len > maximum {
            return Err(OfflineSyncError::SnapshotTooLarge { maximum });
        }

        let mut envelope = Vec::with_capacity(encoded_len);
        envelope.extend_from_slice(ENVELOPE_MAGIC);
        envelope.push(self.key_id.len() as u8);
        envelope.extend_from_slice(self.key_id.as_bytes());
        envelope.extend_from_slice(&nonce);
        envelope.extend_from_slice(&ciphertext);
        Ok(envelope)
    }

    /// Authenticates, decrypts, and validates one snapshot for the expected account.
    pub fn open(
        &self,
        policy: OfflineSyncPolicy,
        expected_account: &OfflineAccountId,
        envelope: &[u8],
    ) -> Result<OfflineSyncState, OfflineSyncError> {
        let maximum = maximum_envelope_bytes(policy)?;
        if envelope.len() > maximum {
            return Err(OfflineSyncError::SnapshotTooLarge { maximum });
        }
        if envelope.len() < FIXED_ENVELOPE_BYTES {
            return Err(OfflineSyncError::InvalidSnapshot);
        }
        if envelope.get(..ENVELOPE_MAGIC.len()) != Some(ENVELOPE_MAGIC.as_slice()) {
            return Err(OfflineSyncError::InvalidSnapshot);
        }
        let key_id_length = envelope
            .get(ENVELOPE_MAGIC.len())
            .copied()
            .map(usize::from)
            .ok_or(OfflineSyncError::InvalidSnapshot)?;
        if key_id_length == 0 || key_id_length > MAX_KEY_ID_BYTES {
            return Err(OfflineSyncError::InvalidSnapshot);
        }
        let key_id_start = ENVELOPE_MAGIC.len() + 1;
        let key_id_end = key_id_start
            .checked_add(key_id_length)
            .ok_or(OfflineSyncError::InvalidSnapshot)?;
        let key_id = std::str::from_utf8(
            envelope
                .get(key_id_start..key_id_end)
                .ok_or(OfflineSyncError::InvalidSnapshot)?,
        )
        .map_err(|_| OfflineSyncError::InvalidSnapshot)?;
        if !valid_key_id(key_id) {
            return Err(OfflineSyncError::InvalidSnapshot);
        }
        if key_id != self.key_id {
            return Err(OfflineSyncError::SnapshotKeyIdMismatch);
        }
        let nonce_end = key_id_end
            .checked_add(NONCE_LENGTH)
            .ok_or(OfflineSyncError::InvalidSnapshot)?;
        let nonce = Nonce::<Aes256Gcm>::try_from(
            envelope
                .get(key_id_end..nonce_end)
                .ok_or(OfflineSyncError::InvalidSnapshot)?,
        )
        .map_err(|_| OfflineSyncError::InvalidSnapshot)?;
        let ciphertext = envelope
            .get(nonce_end..)
            .filter(|ciphertext| ciphertext.len() >= TAG_LENGTH)
            .ok_or(OfflineSyncError::InvalidSnapshot)?;
        let aad = authenticated_data(key_id, expected_account);
        let cipher = Aes256Gcm::new_from_slice(self.key.as_ref())
            .map_err(|_| OfflineSyncError::InvalidKeyLength)?;
        let plaintext = cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: ciphertext,
                    aad: &aad,
                },
            )
            .map(Zeroizing::new)
            .map_err(|_| OfflineSyncError::AuthenticationFailed)?;
        let state = OfflineSyncState::decode(policy, plaintext.as_slice())?;
        if state.account_id() != expected_account {
            return Err(OfflineSyncError::AccountMismatch);
        }
        Ok(state)
    }
}

impl fmt::Debug for OfflineSnapshotCipher {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OfflineSnapshotCipher")
            .field("key_id", &self.key_id)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

fn authenticated_data(key_id: &str, account: &OfflineAccountId) -> Vec<u8> {
    let mut aad = Vec::with_capacity(
        OFFLINE_SNAPSHOT_DOMAIN.len() + key_id.len() + account.as_str().len() + 2,
    );
    aad.extend_from_slice(OFFLINE_SNAPSHOT_DOMAIN.as_bytes());
    aad.push(0);
    aad.extend_from_slice(key_id.as_bytes());
    aad.push(0);
    aad.extend_from_slice(account.as_str().as_bytes());
    aad
}

fn maximum_envelope_bytes(policy: OfflineSyncPolicy) -> Result<usize, OfflineSyncError> {
    policy
        .max_snapshot_bytes()
        .checked_add(FIXED_ENVELOPE_BYTES)
        .and_then(|length| length.checked_add(MAX_KEY_ID_BYTES))
        .ok_or(OfflineSyncError::SnapshotTooLarge {
            maximum: policy.max_snapshot_bytes(),
        })
}

fn valid_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_KEY_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
