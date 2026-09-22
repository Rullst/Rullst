use super::{MultipartError, MultipartStorage, State};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use std::{
    fmt,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

const MAX_TOKEN: usize = 256 * 1024;

/// Independent AES-256-GCM key for durable recovery records; Debug is redacted.
/// Retain the old key until its uploads are completed/aborted during rotation.
#[derive(Clone)]
pub struct MultipartKey(Arc<aead::LessSafeKey>);

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
        let key = aead::UnboundKey::new(&aead::AES_256_GCM, &bytes)
            .map_err(|_| MultipartError::InvalidInput);
        Ok(Self(Arc::new(aead::LessSafeKey::new(key?))))
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

    fn storage() -> MultipartStorage {
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
}

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
        let mut bytes = serde_json::to_vec(state).map_err(|_| MultipartError::InvalidCheckpoint)?;
        let mut nonce = [0; 12];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| MultipartError::InvalidCheckpoint)?;
        self.key
            .0
            .seal_in_place_append_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(self.binding),
                &mut bytes,
            )
            .map_err(|_| MultipartError::InvalidCheckpoint)?;
        let mut token = nonce.to_vec();
        token.extend(bytes);
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
        let mut bytes = URL_SAFE_NO_PAD
            .decode(text)
            .map_err(|_| MultipartError::InvalidCheckpoint)?;
        if bytes.len() < 28 || URL_SAFE_NO_PAD.encode(&bytes) != text {
            return Err(MultipartError::InvalidCheckpoint);
        }
        let (nonce, body) = bytes.split_at_mut(12);
        let nonce: [u8; 12] = nonce
            .try_into()
            .map_err(|_| MultipartError::InvalidCheckpoint)?;
        let body = self
            .key
            .0
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(self.binding),
                body,
            )
            .map_err(|_| MultipartError::InvalidCheckpoint)?;
        let state: State =
            serde_json::from_slice(body).map_err(|_| MultipartError::InvalidCheckpoint)?;
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
