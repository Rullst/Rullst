//! Bounded, authenticated document export and crash-resumable restoration.
//!
//! This module provides an application-operated recovery primitive. It does not
//! claim an atomic cross-store transaction, online replication, deletion of
//! destination data, or vendor-managed backup durability.

mod codec;
mod operation;

#[cfg(test)]
#[path = "recovery/tests.rs"]
mod tests;

use std::fmt;

use aes_gcm::{Aes256Gcm, KeyInit};
use zeroize::Zeroizing;

use super::{CollectionName, PolyglotError};

const MAX_APPLICATION_NAMESPACE_BYTES: usize = 128;
const MAX_KEY_ID_BYTES: usize = 64;
const MIN_SNAPSHOT_BYTES: usize = 1_024;
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
const MAX_DOCUMENTS: u32 = 100_000;

/// Typed failures that omit document contents, identifiers, keys and paths.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DocumentRecoveryError {
    /// Key identifiers and key material must meet the fixed contract.
    #[error("document recovery key must have a portable ID and exactly 32 bytes")]
    InvalidKey,
    /// The application namespace is missing, oversized or non-portable.
    #[error("document recovery application namespace is invalid")]
    InvalidBinding,
    /// A policy field exceeded its documented bound.
    #[error("document recovery policy is invalid")]
    InvalidPolicy,
    /// The serialized envelope is malformed or exceeds its configured bound.
    #[error("document recovery envelope is malformed or oversized")]
    InvalidEnvelope,
    /// The envelope version is not implemented by this release.
    #[error("document recovery envelope version is unsupported")]
    UnsupportedVersion,
    /// The caller supplied a different rotation key.
    #[error("document recovery snapshot requires a different key ID")]
    KeyIdMismatch,
    /// Random nonce generation was unavailable.
    #[error("operating-system randomness is unavailable")]
    RandomnessUnavailable,
    /// Encryption failed without exposing document contents.
    #[error("document recovery snapshot encryption failed")]
    EncryptionFailed,
    /// Key, binding, nonce, ciphertext or tag did not authenticate.
    #[error("document recovery snapshot authentication failed")]
    AuthenticationFailed,
    /// Authenticated plaintext violated the versioned portable schema.
    #[error("document recovery snapshot payload is invalid")]
    InvalidPayload,
    /// The model is not portable across the supported document adapters.
    #[error("document recovery requires JSON-object models without id or _id fields")]
    NonPortableModel,
    /// A configured document or byte ceiling was exhausted.
    #[error("document recovery capacity was exceeded")]
    CapacityExceeded,
    /// An adapter violated stable, strictly increasing inventory order.
    #[error("document recovery inventory was not strictly ordered")]
    InvalidInventory,
    /// The source changed between the two export observations.
    #[error("document recovery source changed during export")]
    SourceChanged,
    /// Existing destination content is extra or differs from the snapshot.
    #[error("document recovery destination conflicts with the snapshot")]
    DestinationConflict,
    /// Destination content changed while restoration was running.
    #[error("document recovery destination changed during restoration")]
    DestinationChanged,
    /// A repository operation failed; its source remains available to callers.
    #[error("document repository operation failed")]
    Repository(#[source] PolyglotError),
}

impl From<PolyglotError> for DocumentRecoveryError {
    fn from(error: PolyglotError) -> Self {
        Self::Repository(error)
    }
}

/// Trusted application and collection scope authenticated with every snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentRecoveryBinding {
    application_namespace: String,
    collection: CollectionName,
}

impl DocumentRecoveryBinding {
    /// Creates a scope from trusted application configuration.
    pub fn try_new(
        application_namespace: impl Into<String>,
        collection: CollectionName,
    ) -> Result<Self, DocumentRecoveryError> {
        let application_namespace = application_namespace.into();
        if !valid_portable_label(&application_namespace, MAX_APPLICATION_NAMESPACE_BYTES) {
            return Err(DocumentRecoveryError::InvalidBinding);
        }
        Ok(Self {
            application_namespace,
            collection,
        })
    }

    /// Returns the trusted application namespace.
    pub fn application_namespace(&self) -> &str {
        &self.application_namespace
    }

    /// Returns the collection authenticated by the snapshot.
    pub fn collection(&self) -> &CollectionName {
        &self.collection
    }
}

/// Explicit memory and pagination limits for one recovery operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentRecoveryPolicy {
    page_size: u32,
    max_documents: u32,
    max_snapshot_bytes: usize,
}

impl DocumentRecoveryPolicy {
    /// Validates policy bounds before any repository access or decoding.
    pub fn try_new(
        page_size: u32,
        max_documents: u32,
        max_snapshot_bytes: usize,
    ) -> Result<Self, DocumentRecoveryError> {
        if page_size == 0
            || page_size > 500
            || max_documents == 0
            || max_documents > MAX_DOCUMENTS
            || !(MIN_SNAPSHOT_BYTES..=MAX_SNAPSHOT_BYTES).contains(&max_snapshot_bytes)
        {
            return Err(DocumentRecoveryError::InvalidPolicy);
        }
        Ok(Self {
            page_size,
            max_documents,
            max_snapshot_bytes,
        })
    }

    /// Returns the per-query document ceiling.
    pub const fn page_size(self) -> u32 {
        self.page_size
    }

    /// Returns the total document ceiling.
    pub const fn max_documents(self) -> u32 {
        self.max_documents
    }

    /// Returns the authenticated plaintext byte ceiling.
    pub const fn max_snapshot_bytes(self) -> usize {
        self.max_snapshot_bytes
    }
}

impl Default for DocumentRecoveryPolicy {
    fn default() -> Self {
        Self {
            page_size: 100,
            max_documents: 10_000,
            max_snapshot_bytes: 16 * 1024 * 1024,
        }
    }
}

/// Named AES-256-GCM key used to encrypt and authenticate snapshots.
pub struct DocumentRecoveryKey {
    key_id: String,
    cipher: Aes256Gcm,
}

impl DocumentRecoveryKey {
    /// Builds a key from 256 bits supplied by a CSPRNG or secret manager.
    pub fn try_new(
        key_id: impl Into<String>,
        key: [u8; 32],
    ) -> Result<Self, DocumentRecoveryError> {
        let key_id = key_id.into();
        if !valid_portable_label(&key_id, MAX_KEY_ID_BYTES) {
            return Err(DocumentRecoveryError::InvalidKey);
        }
        let key = Zeroizing::new(key);
        let cipher = Aes256Gcm::new_from_slice(key.as_ref())
            .map_err(|_| DocumentRecoveryError::EncryptionFailed)?;
        Ok(Self { key_id, cipher })
    }

    /// Returns the non-secret key-rotation identifier.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

impl fmt::Debug for DocumentRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DocumentRecoveryKey")
            .field("key_id", &self.key_id)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

/// Opaque encrypted snapshot suitable for application-owned durable storage.
#[derive(Clone, PartialEq, Eq)]
pub struct EncryptedDocumentSnapshot(String);

impl EncryptedDocumentSnapshot {
    /// Revalidates an envelope loaded from storage before retaining it.
    pub fn try_from_envelope(
        envelope: impl Into<String>,
        policy: DocumentRecoveryPolicy,
    ) -> Result<Self, DocumentRecoveryError> {
        let envelope = envelope.into();
        codec::validate_envelope(&envelope, policy)?;
        Ok(Self(envelope))
    }

    /// Returns the opaque envelope at an explicit persistence boundary.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for EncryptedDocumentSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EncryptedDocumentSnapshot([REDACTED])")
    }
}

/// Counts from one verified restoration attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentRecoveryReport {
    inserted: u32,
    replayed: u32,
    verified: u32,
}

impl DocumentRecoveryReport {
    /// Returns documents newly inserted during this attempt.
    pub const fn inserted(self) -> u32 {
        self.inserted
    }

    /// Returns matching documents already present or won by a concurrent retry.
    pub const fn replayed(self) -> u32 {
        self.replayed
    }

    /// Returns the final exact inventory size.
    pub const fn verified(self) -> u32 {
        self.verified
    }
}

fn valid_portable_label(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

pub use operation::{export_document_snapshot, restore_document_snapshot};
