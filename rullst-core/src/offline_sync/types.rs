use super::json_limit::{BoundedJsonError, validate_bounded};
use super::policy::MAX_OFFLINE_PAYLOAD_BYTES;
use super::{OfflineSyncError, OfflineSyncPolicy};
use crate::client_contract::{FailureCode, IdempotencyKey};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

const MAX_ACCOUNT_ID_BYTES: usize = 128;
const MAX_COLLECTION_BYTES: usize = 64;
const MAX_ENTITY_ID_BYTES: usize = 128;
const MAX_CURSOR_BYTES: usize = 256;

/// Authenticated application account to which one offline state is bound.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
#[non_exhaustive]
pub struct OfflineAccountId(String);

impl OfflineAccountId {
    /// Validates a portable account identifier without granting it authority.
    pub fn new(value: impl Into<String>) -> Result<Self, OfflineSyncError> {
        let value = value.into();
        if valid_token(&value, MAX_ACCOUNT_ID_BYTES) {
            Ok(Self(value))
        } else {
            Err(OfflineSyncError::InvalidAccountId)
        }
    }

    /// Returns the validated account identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for OfflineAccountId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// Stable application collection plus entity identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct OfflineEntityKey {
    collection: String,
    entity_id: String,
}

impl OfflineEntityKey {
    /// Creates a bounded portable entity key.
    pub fn new(
        collection: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Result<Self, OfflineSyncError> {
        let collection = collection.into();
        let entity_id = entity_id.into();
        if !valid_token(&collection, MAX_COLLECTION_BYTES) {
            return Err(OfflineSyncError::InvalidCollection);
        }
        if !valid_token(&entity_id, MAX_ENTITY_ID_BYTES) {
            return Err(OfflineSyncError::InvalidEntityId);
        }
        Ok(Self {
            collection,
            entity_id,
        })
    }

    /// Returns the application collection.
    pub fn collection(&self) -> &str {
        &self.collection
    }

    /// Returns the entity identifier.
    pub fn entity_id(&self) -> &str {
        &self.entity_id
    }
}

impl<'de> Deserialize<'de> for OfflineEntityKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireKey {
            collection: String,
            entity_id: String,
        }
        let wire = WireKey::deserialize(deserializer)?;
        Self::new(wire.collection, wire.entity_id).map_err(D::Error::custom)
    }
}

/// Opaque, server-authored incremental synchronization cursor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
#[non_exhaustive]
pub struct SyncCursor(String);

impl SyncCursor {
    /// Validates a bounded opaque cursor token.
    pub fn new(value: impl Into<String>) -> Result<Self, OfflineSyncError> {
        let value = value.into();
        if valid_token(&value, MAX_CURSOR_BYTES) {
            Ok(Self(value))
        } else {
            Err(OfflineSyncError::InvalidCursor)
        }
    }

    /// Returns the opaque cursor without interpreting it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SyncCursor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// Server-authoritative entity value or tombstone.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
#[non_exhaustive]
pub enum OfflineRecordValue {
    /// Current server-authored JSON representation.
    Upsert(Value),
    /// Server-authored deletion tombstone retaining its revision.
    Deleted,
}

impl std::fmt::Debug for OfflineRecordValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Upsert(_) => formatter.write_str("Upsert([REDACTED])"),
            Self::Deleted => formatter.write_str("Deleted"),
        }
    }
}

/// Versioned server record cached for offline reads and conflict detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ServerRecord {
    pub(super) key: OfflineEntityKey,
    pub(super) revision: u64,
    pub(super) value: OfflineRecordValue,
}

impl ServerRecord {
    /// Creates a positive-revision server record with a hard-bounded payload.
    pub fn new(
        key: OfflineEntityKey,
        revision: u64,
        value: OfflineRecordValue,
    ) -> Result<Self, OfflineSyncError> {
        if revision == 0 {
            return Err(OfflineSyncError::InvalidRevision);
        }
        if let OfflineRecordValue::Upsert(payload) = &value {
            validate_payload(payload, MAX_OFFLINE_PAYLOAD_BYTES)?;
        }
        Ok(Self {
            key,
            revision,
            value,
        })
    }

    /// Returns the entity key.
    pub fn key(&self) -> &OfflineEntityKey {
        &self.key
    }

    /// Returns the monotonically increasing server revision.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the server-authored value or tombstone.
    pub const fn value(&self) -> &OfflineRecordValue {
        &self.value
    }
}

/// Client mutation operation; the server must revalidate its payload and policy.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
#[non_exhaustive]
pub enum OfflineMutationKind {
    /// Proposes a JSON upsert.
    Upsert(Value),
    /// Proposes deletion of the entity.
    Delete,
}

impl std::fmt::Debug for OfflineMutationKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Upsert(_) => formatter.write_str("Upsert([REDACTED])"),
            Self::Delete => formatter.write_str("Delete"),
        }
    }
}

/// One replay-safe locally queued mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct OfflineMutation {
    pub(super) idempotency_key: IdempotencyKey,
    pub(super) entity: OfflineEntityKey,
    pub(super) base_revision: Option<u64>,
    pub(super) queued_epoch_ms: u64,
    pub(super) operation: OfflineMutationKind,
}

impl OfflineMutation {
    /// Creates a bounded local upsert proposal.
    pub fn upsert(
        idempotency_key: IdempotencyKey,
        entity: OfflineEntityKey,
        base_revision: Option<u64>,
        queued_epoch_ms: u64,
        payload: Value,
    ) -> Result<Self, OfflineSyncError> {
        validate_payload(&payload, MAX_OFFLINE_PAYLOAD_BYTES)?;
        Ok(Self {
            idempotency_key,
            entity,
            base_revision,
            queued_epoch_ms,
            operation: OfflineMutationKind::Upsert(payload),
        })
    }

    /// Creates a local deletion proposal.
    pub const fn delete(
        idempotency_key: IdempotencyKey,
        entity: OfflineEntityKey,
        base_revision: Option<u64>,
        queued_epoch_ms: u64,
    ) -> Self {
        Self {
            idempotency_key,
            entity,
            base_revision,
            queued_epoch_ms,
            operation: OfflineMutationKind::Delete,
        }
    }

    /// Returns the stable replay key.
    pub const fn idempotency_key(&self) -> &IdempotencyKey {
        &self.idempotency_key
    }

    /// Returns the target entity.
    pub const fn entity(&self) -> &OfflineEntityKey {
        &self.entity
    }

    /// Returns the server revision observed before the local edit, if any.
    pub const fn base_revision(&self) -> Option<u64> {
        self.base_revision
    }

    /// Returns untrusted client time for UX ordering only.
    pub const fn queued_epoch_ms(&self) -> u64 {
        self.queued_epoch_ms
    }

    /// Returns the proposed operation.
    pub const fn operation(&self) -> &OfflineMutationKind {
        &self.operation
    }

    pub(super) fn replay_from(
        &self,
        idempotency_key: IdempotencyKey,
        base_revision: Option<u64>,
    ) -> Self {
        Self {
            idempotency_key,
            entity: self.entity.clone(),
            base_revision,
            queued_epoch_ms: self.queued_epoch_ms,
            operation: self.operation.clone(),
        }
    }
}

/// Ordered client push payload generated from the pending queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SyncPushBatch {
    pub(super) cursor: Option<SyncCursor>,
    pub(super) mutations: Vec<OfflineMutation>,
}

impl SyncPushBatch {
    /// Returns the last committed server cursor, if known.
    pub const fn cursor(&self) -> Option<&SyncCursor> {
        self.cursor.as_ref()
    }

    /// Returns queued mutations in stable FIFO order.
    pub fn mutations(&self) -> &[OfflineMutation] {
        &self.mutations
    }
}

/// Server decision for one queued mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
#[non_exhaustive]
pub enum MutationOutcome {
    /// The server durably accepted the replay key and returned current state.
    Applied {
        /// Replay key being acknowledged.
        idempotency_key: IdempotencyKey,
        /// Current authoritative record, including deletion tombstones.
        record: ServerRecord,
    },
    /// The mutation conflicts with current authoritative state.
    Conflict {
        /// Replay key being removed from the active queue.
        idempotency_key: IdempotencyKey,
        /// Current authoritative record.
        server_record: ServerRecord,
        /// Stable application conflict code.
        code: FailureCode,
    },
    /// The server rejected the mutation without arbitrary debug text.
    Rejected {
        /// Replay key being decided.
        idempotency_key: IdempotencyKey,
        /// Stable application rejection code.
        code: FailureCode,
        /// Whether a later retry of the same replay key may succeed.
        retryable: bool,
    },
}

impl MutationOutcome {
    pub(super) const fn idempotency_key(&self) -> &IdempotencyKey {
        match self {
            Self::Applied {
                idempotency_key, ..
            }
            | Self::Conflict {
                idempotency_key, ..
            }
            | Self::Rejected {
                idempotency_key, ..
            } => idempotency_key,
        }
    }
}

/// Server response for a client push batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SyncPushResult {
    pub(super) cursor: Option<SyncCursor>,
    pub(super) outcomes: Vec<MutationOutcome>,
}

impl SyncPushResult {
    /// Creates a typed server push result.
    pub fn new(cursor: Option<SyncCursor>, outcomes: Vec<MutationOutcome>) -> Self {
        Self { cursor, outcomes }
    }

    /// Returns the next committed cursor, if supplied.
    pub const fn cursor(&self) -> Option<&SyncCursor> {
        self.cursor.as_ref()
    }

    /// Returns per-mutation decisions.
    pub fn outcomes(&self) -> &[MutationOutcome] {
        &self.outcomes
    }
}

/// One bounded page of server-authored changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SyncPullPage {
    pub(super) cursor: SyncCursor,
    pub(super) changes: Vec<ServerRecord>,
    pub(super) has_more: bool,
    pub(super) requires_full_resync: bool,
}

impl SyncPullPage {
    /// Creates a typed incremental page or an explicit resync signal.
    pub fn new(
        cursor: SyncCursor,
        changes: Vec<ServerRecord>,
        has_more: bool,
        requires_full_resync: bool,
    ) -> Self {
        Self {
            cursor,
            changes,
            has_more,
            requires_full_resync,
        }
    }

    /// Returns the next server cursor.
    pub const fn cursor(&self) -> &SyncCursor {
        &self.cursor
    }

    /// Returns the authoritative changes.
    pub fn changes(&self) -> &[ServerRecord] {
        &self.changes
    }

    /// Reports whether another page should be fetched immediately.
    pub const fn has_more(&self) -> bool {
        self.has_more
    }

    /// Reports that incremental history is no longer sufficient.
    pub const fn requires_full_resync(&self) -> bool {
        self.requires_full_resync
    }
}

pub(super) fn validate_payload(payload: &Value, maximum: usize) -> Result<(), OfflineSyncError> {
    match validate_bounded(payload, maximum) {
        Ok(()) => Ok(()),
        Err(BoundedJsonError::LimitExceeded) => Err(OfflineSyncError::PayloadTooLarge { maximum }),
        Err(BoundedJsonError::Encode(error)) => Err(OfflineSyncError::EncodeJson(error)),
    }
}

pub(super) fn validate_record(
    record: &ServerRecord,
    policy: OfflineSyncPolicy,
) -> Result<(), OfflineSyncError> {
    if record.revision == 0 {
        return Err(OfflineSyncError::InvalidRevision);
    }
    if let OfflineRecordValue::Upsert(payload) = &record.value {
        validate_payload(payload, policy.max_payload_bytes())?;
    }
    Ok(())
}

pub(super) fn validate_mutation(
    mutation: &OfflineMutation,
    policy: OfflineSyncPolicy,
) -> Result<(), OfflineSyncError> {
    if let Some(0) = mutation.base_revision {
        return Err(OfflineSyncError::InvalidRevision);
    }
    if let OfflineMutationKind::Upsert(payload) = &mutation.operation {
        validate_payload(payload, policy.max_payload_bytes())?;
    }
    Ok(())
}

fn valid_token(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}
