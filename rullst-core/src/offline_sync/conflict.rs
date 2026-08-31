use super::OfflineMutation;
use crate::client_contract::{FailureCode, IdempotencyKey};
use serde::{Deserialize, Serialize};

/// Why a local mutation needs an explicit application/user decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "code", rename_all = "snake_case")]
#[non_exhaustive]
pub enum OfflineConflictReason {
    /// The server explicitly reported a domain conflict.
    ServerConflict(FailureCode),
    /// An incremental pull advanced the same entity after the local edit base.
    ConcurrentServerChange,
    /// A complete resync no longer matches the local edit base.
    FullResyncDivergence,
    /// The server permanently rejected the mutation.
    PermanentRejection(FailureCode),
}

/// Local mutation isolated from automatic replay after a conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct OfflineConflict {
    pub(super) mutation: OfflineMutation,
    pub(super) authoritative_revision: Option<u64>,
    pub(super) reason: OfflineConflictReason,
}

impl OfflineConflict {
    /// Returns the isolated local mutation.
    pub const fn mutation(&self) -> &OfflineMutation {
        &self.mutation
    }

    /// Returns the latest server revision known for the entity, if present.
    ///
    /// Read the value itself through [`OfflineSyncState::record`](super::OfflineSyncState::record)
    /// so conflicts do not duplicate potentially sensitive payloads.
    pub const fn authoritative_revision(&self) -> Option<u64> {
        self.authoritative_revision
    }

    /// Returns the machine-readable conflict reason.
    pub const fn reason(&self) -> &OfflineConflictReason {
        &self.reason
    }
}

/// Explicit conflict decision; there is no implicit client-wins mode.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConflictResolution {
    /// Keep authoritative server state and discard the isolated proposal.
    AcceptServer,
    /// Requeue the original proposal against current server state with a new replay key.
    RetryAgainstServer {
        /// New replay key; reusing the decided key is forbidden.
        idempotency_key: IdempotencyKey,
    },
}

/// Counts returned after dropping derived server cache for a clean pull.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RecoverySummary {
    /// Number of cached server records removed.
    pub records_cleared: usize,
    /// Local pending mutations retained.
    pub pending_preserved: usize,
    /// Conflicts retained for explicit resolution.
    pub conflicts_preserved: usize,
}

/// Counts returned after logical account erasure from this state object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ErasureSummary {
    /// Cached server records removed.
    pub records_erased: usize,
    /// Pending local mutations removed.
    pub pending_erased: usize,
    /// Conflict entries removed.
    pub conflicts_erased: usize,
}
