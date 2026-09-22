//! Bounded private multipart uploads with authenticated resumable checkpoints.
mod codec;
mod operations;
pub(crate) mod protocol;
pub(crate) mod transport;

use super::{CloudClient, CloudError};
use crate::storage::{Storage, StorageError, TenantStorage};
pub use codec::{MultipartCheckpoint, MultipartKey};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, sync::Arc, time::Duration};

/// Multipart failures never include checkpoint contents, object keys or provider bodies.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MultipartError {
    /// Policy, expected length, part number or part size is invalid.
    #[error("invalid multipart limits or part")]
    InvalidInput,
    /// Checkpoint authentication, resource binding or format failed.
    #[error("invalid multipart checkpoint")]
    InvalidCheckpoint,
    /// The checkpoint is expired or the clock precedes its creation.
    #[error("multipart checkpoint is outside its valid lifetime")]
    Expired,
    /// Caller-supplied SHA-256 does not match the uploaded bytes.
    #[error("multipart checksum mismatch")]
    Checksum,
    /// Completion requires all consecutive, size-matched part receipts.
    #[error("multipart upload is incomplete")]
    Incomplete,
    /// Provider response is malformed or contradicts the requested resource.
    #[error("invalid multipart provider response")]
    InvalidResponse,
    /// Completion may have succeeded; explicitly reconcile before retrying.
    #[error("multipart completion acknowledgement is uncertain")]
    CompletionUncertain,
    /// Cloud transport, credential or status failure.
    #[error(transparent)]
    Cloud(#[from] CloudError),
}

/// Fixed per-uploader resource budget, also authenticated into checkpoints.
#[derive(Clone, Debug)]
pub struct MultipartLimits {
    pub(super) max_total: u64,
    pub(super) part_bytes: u32,
    pub(super) lifetime: Duration,
}

impl MultipartLimits {
    /// Allows up to 256 parts of 5–64 MiB; total is capped at 16 GiB.
    /// Lifetime must be an exact number of seconds between one minute and seven days.
    pub fn new(
        max_total_bytes: u64,
        part_bytes: u32,
        lifetime: Duration,
    ) -> Result<Self, MultipartError> {
        if !(5 * 1024 * 1024..=64 * 1024 * 1024).contains(&part_bytes)
            || max_total_bytes == 0
            || max_total_bytes > u64::from(part_bytes) * 256
            || !(60..=604800).contains(&lifetime.as_secs())
            || lifetime.subsec_nanos() != 0
        {
            return Err(MultipartError::InvalidInput);
        }
        Ok(Self {
            max_total: max_total_bytes,
            part_bytes,
            lifetime,
        })
    }
}

/// One server-approved object, policy and independent checkpoint-encryption key.
/// Applications authorize each call and durably serialize/CAS checkpoint updates.
#[derive(Clone)]
pub struct MultipartStorage {
    client: Arc<CloudClient>,
    object: String,
    key: MultipartKey,
    limits: MultipartLimits,
    binding: [u8; 32],
}

impl fmt::Debug for MultipartStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultipartStorage")
            .field("mock", &self.client.is_mock())
            .finish_non_exhaustive()
    }
}

impl Storage {
    /// Binds multipart operations to one approved object in the configured private backend.
    pub fn multipart(
        &self,
        object: impl Into<String>,
        key: MultipartKey,
        limits: MultipartLimits,
    ) -> Result<MultipartStorage, StorageError> {
        let object = object.into();
        super::validate_key(&object)?;
        let client = self.cloud.clone().ok_or_else(|| {
            StorageError::Unsupported("multipart requires configured cloud storage".into())
        })?;
        let binding = client.multipart_binding(&object, &limits);
        Ok(MultipartStorage {
            client,
            object,
            key,
            limits,
            binding,
        })
    }
}

impl TenantStorage {
    /// Derives the immutable tenant/object binding from the authenticated tenant context.
    pub fn multipart(
        &self,
        object: impl Into<String>,
        key: MultipartKey,
        limits: MultipartLimits,
    ) -> Result<MultipartStorage, StorageError> {
        self.storage
            .multipart(self.object_key(&object.into())?, key, limits)
    }
}

/// Remote part observation; a false receipt match requires explicit re-upload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultipartPart {
    /// Consecutive, one-based part number.
    pub number: u16,
    /// Provider-observed byte count.
    pub size_bytes: u64,
    /// Whether the provider ETag/length agrees with the authenticated checkpoint.
    pub matches_checkpoint: bool,
}

/// Outcome of checking a possibly lost completion response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionStatus {
    /// Exact upload marker and expected object length were observed.
    Confirmed,
    /// No object with both the expected marker and size was observed.
    Unconfirmed,
}

/// Result of abort followed by a provider existence check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbortStatus {
    /// Provider reports this upload no longer exists; already completed objects are untouched.
    Gone,
    /// Upload is still observable; stop writers and retry abort.
    RetryRequired,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    upload: String,
    marker: String,
    total: u64,
    created: u64,
    expires: u64,
    receipts: BTreeMap<u16, Receipt>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    etag: String,
    size: u64,
    sha256: [u8; 32],
}

impl MultipartStorage {
    fn part_count(&self, state: &State) -> u16 {
        state.total.div_ceil(u64::from(self.limits.part_bytes)) as u16
    }
    fn part_size(&self, state: &State, number: u16) -> Result<u64, MultipartError> {
        if number == 0 || number > self.part_count(state) {
            return Err(MultipartError::InvalidInput);
        }
        Ok(
            (state.total - u64::from(number - 1) * u64::from(self.limits.part_bytes))
                .min(u64::from(self.limits.part_bytes)),
        )
    }
}
