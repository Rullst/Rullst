use super::{MultipartError, MultipartStorage, State};
use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, Generate, Nonce, Payload},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{
    fmt,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

const MAX_TOKEN: usize = 256 * 1024;
const NONCE_BYTES: usize = 12;

/// Independent AES-256-GCM key for durable recovery records; Debug is redacted.
/// Retain the old key until its uploads are completed/aborted during rotation.
#[derive(Clone)]
pub struct MultipartKey(Arc<Aes256Gcm>);

impl MultipartKey {
    /// Accepts exactly 32 random bytes, encoded as canonical unpadded base64url.
    /// This key is independent from provider credentials; mocks do not weaken it.
    pub fn new(encoded: impl Into<String>) -> Result<Self, MultipartError> {
        let encoded = Zeroizing::new(encoded.into());
        if encoded.len() != 43 {
            return Err(MultipartError::InvalidInput);
        }
        let bytes = Zeroizing::new(
            URL_SAFE_NO_PAD
                .decode(encoded.as_bytes())
                .map_err(|_| MultipartError::InvalidInput)?,
        );
        if bytes.len() != 32 || URL_SAFE_NO_PAD.encode(&*bytes) != *encoded {
            return Err(MultipartError::InvalidInput);
        }
        // The cipher clears its round keys and GHASH key when dropped.
        let cipher = Aes256Gcm::new_from_slice(&bytes).map_err(|_| MultipartError::InvalidInput)?;
        Ok(Self(Arc::new(cipher)))
    }

    /// The record `nonce || ciphertext || tag` of `plaintext`, authenticated
    /// with `aad`: the layout that `ring` wrote before 13.0.0-alpha.6.
    fn seal(
        &self,
        nonce: [u8; NONCE_BYTES],
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, MultipartError> {
        let payload = Payload {
            msg: plaintext,
            aad,
        };
        let sealed = self
            .0
            .encrypt(&Nonce::<Aes256Gcm>::from(nonce), payload)
            .map_err(|_| MultipartError::InvalidCheckpoint)?;
        Ok([nonce.as_slice(), &sealed].concat())
    }

    /// The plaintext of a record from [`Self::seal`] with the same `aad`.
    fn open(&self, aad: &[u8], record: &[u8]) -> Result<Vec<u8>, MultipartError> {
        let (nonce, sealed) = record
            .split_first_chunk::<NONCE_BYTES>()
            .ok_or(MultipartError::InvalidCheckpoint)?;
        let payload = Payload { msg: sealed, aad };
        self.0
            .decrypt(&Nonce::<Aes256Gcm>::from(*nonce), payload)
            .map_err(|_| MultipartError::InvalidCheckpoint)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::{
        Storage,
        cloud::{CloudCredentials, CloudStorageConfig, multipart::MultipartLimits},
    };
    use std::{collections::BTreeMap, time::Duration};

    pub(super) fn storage() -> MultipartStorage {
        Storage::s3("test-bucket", "auto")
            .with_cloud_config(CloudStorageConfig::new(
                CloudCredentials::new("", "").unwrap(),
            ))
            .unwrap()
            .multipart(
                "private/file",
                MultipartKey::new(URL_SAFE_NO_PAD.encode([17; 32])).unwrap(),
                MultipartLimits::new(10, 5 * 1024 * 1024, Duration::from_secs(60)).unwrap(),
            )
            .unwrap()
    }

    #[test]
    fn expired_records_only_allow_cleanup_and_policy_or_clock_drift_denies() {
        let storage = storage();
        let now = now().unwrap();
        let mut state = State {
            upload: "owned-upload".into(),
            marker: "owned-marker".into(),
            total: 10,
            created: now - 61,
            expires: now - 1,
            receipts: BTreeMap::new(),
        };
        let token = storage.seal(&state).unwrap();
        assert!(matches!(
            storage.open(&token, false),
            Err(MultipartError::Expired)
        ));
        assert!(storage.open(&token, true).is_ok());
        state.created = now + 60;
        state.expires = state.created + 60;
        assert!(matches!(
            storage.open(&storage.seal(&state).unwrap(), true),
            Err(MultipartError::Expired)
        ));
        state.created = now;
        state.expires = now + 61;
        assert!(matches!(
            storage.open(&storage.seal(&state).unwrap(), true),
            Err(MultipartError::InvalidCheckpoint)
        ));
        state.expires = now + 60;
        state.total = 11;
        assert!(matches!(
            storage.open(&storage.seal(&state).unwrap(), true),
            Err(MultipartError::InvalidCheckpoint)
        ));
    }

    #[test]
    fn authenticated_bytes_and_outer_encoding_are_bounded_and_canonical() {
        assert!(
            MultipartCheckpoint::from_encoded("mp1.".to_owned() + &"a".repeat(MAX_TOKEN)).is_err()
        );
        assert!(MultipartKey::new("mock_not_a_key").is_err());
        assert!(MultipartKey::new("".to_string()).is_err());
        let storage = storage();
        for text in [
            "mp1.",
            "mp1.!!",
            "mp1.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            assert!(matches!(
                storage.open(&MultipartCheckpoint::from_encoded(text).unwrap(), true),
                Err(MultipartError::InvalidCheckpoint)
            ));
        }
    }

    #[test]
    fn maximum_receipt_inventory_fits_the_checkpoint_bound() {
        let mut storage = storage();
        storage.limits.max_total = u64::from(storage.limits.part_bytes) * 256;
        storage.binding = storage
            .client
            .multipart_binding(&storage.object, &storage.limits);
        let now = now().unwrap();
        let state = State {
            upload: "a".repeat(2048),
            marker: "marker".into(),
            total: storage.limits.max_total,
            created: now,
            expires: now + 60,
            receipts: (1..=256)
                .map(|number| {
                    (
                        number,
                        super::super::Receipt {
                            etag: format!("\"{}\"", "x".repeat(254)),
                            size: u64::from(storage.limits.part_bytes),
                            sha256: [255; 32],
                        },
                    )
                })
                .collect(),
        };
        let token = storage.seal(&state).unwrap();
        assert!(token.expose_encoded().len() < MAX_TOKEN);
        assert_eq!(storage.open(&token, false).unwrap().receipts.len(), 256);
    }

    #[test]
    fn outer_tokens_may_fill_256_kib_and_debug_output_is_redacted() {
        let longest = format!("mp1.{}", "a".repeat(262_140));
        assert!(MultipartCheckpoint::from_encoded(longest).is_ok());
        let storage = storage();
        assert_eq!(format!("{:?}", storage.key), "MultipartKey([REDACTED])");
        let now = now().unwrap();
        let token = storage
            .seal(&State {
                upload: "owned-upload".into(),
                marker: "owned-marker".into(),
                total: 10,
                created: now,
                expires: now + 60,
                receipts: BTreeMap::new(),
            })
            .unwrap();
        assert_eq!(format!("{token:?}"), "MultipartCheckpoint([REDACTED])");
    }

    #[test]
    fn authenticated_state_needs_a_size_and_protocol_tokens() {
        let storage = storage();
        let now = now().unwrap();
        let valid = State {
            upload: "owned-upload".into(),
            marker: "owned-marker".into(),
            total: 10,
            created: now,
            expires: now + 60,
            receipts: BTreeMap::new(),
        };
        assert!(storage.open(&storage.seal(&valid).unwrap(), false).is_ok());
        let mut invalid = Vec::new();
        let mut empty = valid.clone();
        empty.total = 0;
        invalid.push(empty);
        for upload in [String::new(), "a".repeat(2_049), "owned upload".into()] {
            let mut state = valid.clone();
            state.upload = upload;
            invalid.push(state);
        }
        // The one 10-byte part needs its exact size and a quoted ETag.
        let mut with_receipt = valid.clone();
        with_receipt.receipts.insert(
            1,
            super::super::Receipt {
                etag: "\"etag\"".into(),
                size: 10,
                sha256: [7; 32],
            },
        );
        assert!(
            storage
                .open(&storage.seal(&with_receipt).unwrap(), false)
                .is_ok()
        );
        for (etag, size) in [("unquoted", 10), ("\"etag\"", 9)] {
            let mut state = with_receipt.clone();
            state.receipts.insert(
                1,
                super::super::Receipt {
                    etag: etag.into(),
                    size,
                    sha256: [7; 32],
                },
            );
            invalid.push(state);
        }
        for state in invalid {
            assert!(matches!(
                storage.open(&storage.seal(&state).unwrap(), false),
                Err(MultipartError::InvalidCheckpoint)
            ));
        }
    }
}

#[cfg(test)]
#[path = "codec_aead_tests.rs"]
mod aead_tests;

impl fmt::Debug for MultipartKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MultipartKey([REDACTED])")
    }
}

/// Encrypted, authenticated resume state. Persist atomically after every accepted part.
#[derive(Clone)]
pub struct MultipartCheckpoint(pub(super) String);

impl MultipartCheckpoint {
    /// Parses only the outer size/version. Every operation authenticates the contents again.
    pub fn from_encoded(encoded: impl Into<String>) -> Result<Self, MultipartError> {
        let encoded = encoded.into();
        if encoded.len() > MAX_TOKEN || !encoded.starts_with("mp1.") {
            return Err(MultipartError::InvalidCheckpoint);
        }
        Ok(Self(encoded))
    }
    /// Sensitive recovery token for durable application storage; do not log it.
    pub fn expose_encoded(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for MultipartCheckpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MultipartCheckpoint([REDACTED])")
    }
}

pub(super) fn now() -> Result<u64, MultipartError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_secs())
        .map_err(|_| MultipartError::Expired)
}

impl MultipartStorage {
    pub(super) fn seal(&self, state: &State) -> Result<MultipartCheckpoint, MultipartError> {
        let nonce =
            Nonce::<Aes256Gcm>::try_generate().map_err(|_| MultipartError::InvalidCheckpoint)?;
        self.seal_with_nonce(state, nonce.into())
    }

    fn seal_with_nonce(
        &self,
        state: &State,
        nonce: [u8; NONCE_BYTES],
    ) -> Result<MultipartCheckpoint, MultipartError> {
        let bytes = serde_json::to_vec(state).map_err(|_| MultipartError::InvalidCheckpoint)?;
        let token = self.key.seal(nonce, &self.binding, &bytes)?;
        MultipartCheckpoint::from_encoded(format!("mp1.{}", URL_SAFE_NO_PAD.encode(token)))
    }

    pub(super) fn open(
        &self,
        checkpoint: &MultipartCheckpoint,
        allow_expired: bool,
    ) -> Result<State, MultipartError> {
        let text = checkpoint
            .0
            .strip_prefix("mp1.")
            .ok_or(MultipartError::InvalidCheckpoint)?;
        if checkpoint.0.len() > MAX_TOKEN {
            return Err(MultipartError::InvalidCheckpoint);
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(text)
            .map_err(|_| MultipartError::InvalidCheckpoint)?;
        if bytes.len() < 28 || URL_SAFE_NO_PAD.encode(&bytes) != text {
            return Err(MultipartError::InvalidCheckpoint);
        }
        let body = self.key.open(&self.binding, &bytes)?;
        let state: State =
            serde_json::from_slice(&body).map_err(|_| MultipartError::InvalidCheckpoint)?;
        if state.total == 0
            || state.total > self.limits.max_total
            || state.receipts.len() > 256
            || state.expires.checked_sub(state.created) != Some(self.limits.lifetime.as_secs())
            || !protocol_token(&state.upload, 2048)
            || !protocol_token(&state.marker, 64)
        {
            return Err(MultipartError::InvalidCheckpoint);
        }
        for (number, receipt) in &state.receipts {
            if self.part_size(&state, *number)? != receipt.size
                || !super::protocol::valid_etag(&receipt.etag)
            {
                return Err(MultipartError::InvalidCheckpoint);
            }
        }
        let now = now()?;
        if now < state.created || (!allow_expired && now >= state.expires) {
            return Err(MultipartError::Expired);
        }
        Ok(state)
    }
}

pub(super) fn protocol_token(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && value.bytes().all(|c| c.is_ascii_graphic())
}
