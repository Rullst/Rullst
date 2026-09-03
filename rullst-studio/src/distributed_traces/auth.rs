use super::{TraceBatchV1, TraceIngestionError, validation::MAX_TRACE_BATCH_BYTES};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use rand::{TryRng as _, rngs::SysRng};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

const MAC_DOMAIN: &[u8] = b"rullst.studio.trace-ingestion.v1";
pub(super) const MIN_KEY_BYTES: usize = 32;
pub(super) const MAX_KEY_BYTES: usize = 128;
pub(super) const MAX_SOURCE_BYTES: usize = 64;
pub(super) const NONCE_BYTES: usize = 16;
pub(super) const MAX_CLOCK_SKEW_SECS: u64 = 60;
const MAX_REPLAY_ENTRIES: usize = 4_096;

/// Secret key used to authenticate distributed trace batches.
#[derive(Clone)]
pub struct TraceIngestionKey(Arc<[u8]>);

impl std::fmt::Debug for TraceIngestionKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TraceIngestionKey([REDACTED])")
    }
}

impl TraceIngestionKey {
    /// Copies a caller-provided 32-to-128-byte HMAC key into protected API state.
    ///
    /// The caller must source high-entropy key material from a secret manager or
    /// an operating-system-backed generator.
    pub fn new(key: impl AsRef<[u8]>) -> Result<Self, TraceIngestionError> {
        let key = key.as_ref();
        if !(MIN_KEY_BYTES..=MAX_KEY_BYTES).contains(&key.len()) {
            return Err(TraceIngestionError::InvalidKey);
        }
        Ok(Self(Arc::from(key)))
    }

    fn mac(&self) -> Result<HmacSha256, TraceIngestionError> {
        HmacSha256::new_from_slice(&self.0).map_err(|_| TraceIngestionError::InvalidKey)
    }
}

/// Serialized batch plus the authentication headers required by the receiver.
pub struct SignedTraceBatch {
    body: Vec<u8>,
    source: String,
    timestamp: String,
    nonce: String,
    signature: String,
}

impl std::fmt::Debug for SignedTraceBatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SignedTraceBatch")
            .field("body_bytes", &self.body.len())
            .field("source", &self.source)
            .field("timestamp", &self.timestamp)
            .finish_non_exhaustive()
    }
}

impl SignedTraceBatch {
    /// Encoded JSON body to send without modification.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Authenticated producer header value.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Signed Unix-seconds header value.
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// One-time base64url nonce header value.
    pub fn nonce(&self) -> &str {
        &self.nonce
    }

    /// Base64url HMAC-SHA256 header value.
    pub fn signature(&self) -> &str {
        &self.signature
    }
}

/// Offline signer used by a trace producer before it sends a batch.
#[derive(Clone, Debug)]
pub struct TraceBatchSigner {
    source: String,
    key: TraceIngestionKey,
}

impl TraceBatchSigner {
    /// Creates a signer bound to one exact producer name and key.
    pub fn new(
        source: impl Into<String>,
        key: TraceIngestionKey,
    ) -> Result<Self, TraceIngestionError> {
        let source = source.into();
        validate_source(&source)?;
        Ok(Self { source, key })
    }

    /// Serializes and signs one batch using the current system clock and a
    /// fresh OS-random nonce.
    pub fn sign(&self, batch: &TraceBatchV1) -> Result<SignedTraceBatch, TraceIngestionError> {
        let timestamp = unix_time()?;
        super::validation::validate_batch(batch, timestamp)?;
        let mut nonce = [0_u8; NONCE_BYTES];
        SysRng
            .try_fill_bytes(&mut nonce)
            .map_err(|_| TraceIngestionError::RandomnessUnavailable)?;
        self.sign_at(batch, timestamp, nonce)
    }

    pub(super) fn sign_at(
        &self,
        batch: &TraceBatchV1,
        timestamp: u64,
        nonce: [u8; NONCE_BYTES],
    ) -> Result<SignedTraceBatch, TraceIngestionError> {
        let body = serde_json::to_vec(batch).map_err(|_| TraceIngestionError::InvalidEncoding)?;
        if body.len() > MAX_TRACE_BATCH_BYTES {
            return Err(TraceIngestionError::InvalidBatch);
        }
        let nonce = URL_SAFE_NO_PAD.encode(nonce);
        let signature = sign_fields(&self.key, &self.source, timestamp, &nonce, &body)?;
        Ok(SignedTraceBatch {
            body,
            source: self.source.clone(),
            timestamp: timestamp.to_string(),
            nonce,
            signature,
        })
    }
}

#[derive(Default)]
pub(super) struct ReplayGuard {
    consumed: Mutex<HashMap<[u8; 32], u64>>,
}

impl ReplayGuard {
    pub(super) fn consume(
        &self,
        source: &str,
        nonce: &str,
        timestamp: u64,
        now: u64,
    ) -> Result<(), TraceIngestionError> {
        let mut digest = Sha256::new();
        update_field(&mut digest, source.as_bytes());
        update_field(&mut digest, nonce.as_bytes());
        let identity: [u8; 32] = digest.finalize().into();
        let mut state = self
            .consumed
            .lock()
            .map_err(|_| TraceIngestionError::StoreUnavailable)?;
        let oldest = now.saturating_sub(MAX_CLOCK_SKEW_SECS);
        state.retain(|_, signed_at| *signed_at >= oldest);
        if state.contains_key(&identity) {
            return Err(TraceIngestionError::ReplayDetected);
        }
        if state.len() >= MAX_REPLAY_ENTRIES {
            return Err(TraceIngestionError::StoreUnavailable);
        }
        state.insert(identity, timestamp);
        Ok(())
    }
}

pub(super) fn authenticate(
    key: &TraceIngestionKey,
    source: &str,
    timestamp: &str,
    nonce: &str,
    signature: &str,
    body: &[u8],
    now: u64,
) -> Result<u64, TraceIngestionError> {
    validate_source(source)?;
    let timestamp = timestamp
        .parse::<u64>()
        .map_err(|_| TraceIngestionError::AuthenticationFailed)?;
    if timestamp.abs_diff(now) > MAX_CLOCK_SKEW_SECS {
        return Err(TraceIngestionError::TimestampOutsideWindow);
    }
    let decoded_nonce = URL_SAFE_NO_PAD
        .decode(nonce)
        .map_err(|_| TraceIngestionError::AuthenticationFailed)?;
    if decoded_nonce.len() != NONCE_BYTES {
        return Err(TraceIngestionError::AuthenticationFailed);
    }
    let decoded_signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| TraceIngestionError::AuthenticationFailed)?;
    if decoded_signature.len() != 32 {
        return Err(TraceIngestionError::AuthenticationFailed);
    }
    let mac = canonical_mac(key, source, timestamp, nonce, body)?;
    mac.verify_slice(&decoded_signature)
        .map_err(|_| TraceIngestionError::AuthenticationFailed)?;
    Ok(timestamp)
}

fn sign_fields(
    key: &TraceIngestionKey,
    source: &str,
    timestamp: u64,
    nonce: &str,
    body: &[u8],
) -> Result<String, TraceIngestionError> {
    let mac = canonical_mac(key, source, timestamp, nonce, body)?;
    Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn canonical_mac(
    key: &TraceIngestionKey,
    source: &str,
    timestamp: u64,
    nonce: &str,
    body: &[u8],
) -> Result<HmacSha256, TraceIngestionError> {
    let mut mac = key.mac()?;
    mac.update(MAC_DOMAIN);
    update_mac_field(&mut mac, source.as_bytes());
    mac.update(&timestamp.to_be_bytes());
    update_mac_field(&mut mac, nonce.as_bytes());
    update_mac_field(&mut mac, body);
    Ok(mac)
}

fn update_mac_field(mac: &mut HmacSha256, value: &[u8]) {
    mac.update(&(value.len() as u64).to_be_bytes());
    mac.update(value);
}

fn update_field(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}

pub(super) fn validate_source(source: &str) -> Result<(), TraceIngestionError> {
    if source.is_empty()
        || source.len() > MAX_SOURCE_BYTES
        || !source
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(TraceIngestionError::AuthenticationFailed);
    }
    Ok(())
}

pub(super) fn unix_time() -> Result<u64, TraceIngestionError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| TraceIngestionError::ClockUnavailable)
}
