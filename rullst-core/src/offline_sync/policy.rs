use super::OfflineSyncError;

/// Absolute number of records one state may cache.
pub const MAX_OFFLINE_RECORDS: usize = 50_000;
/// Absolute number of pending mutations one state may retain.
pub const MAX_OFFLINE_PENDING_MUTATIONS: usize = 2_000;
/// Absolute number of unresolved conflicts one state may retain.
pub const MAX_OFFLINE_CONFLICTS: usize = 2_000;
/// Absolute JSON payload ceiling per entity or mutation: 512 KiB.
pub const MAX_OFFLINE_PAYLOAD_BYTES: usize = 512 * 1024;
/// Absolute plaintext snapshot ceiling: 32 MiB.
pub const MAX_OFFLINE_SNAPSHOT_BYTES: usize = 32 * 1024 * 1024;
/// Absolute mutations per push request.
pub const MAX_OFFLINE_PUSH_BATCH: usize = 100;

/// Resource policy applied before offline state is admitted or decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OfflineSyncPolicy {
    max_records: usize,
    max_pending_mutations: usize,
    max_conflicts: usize,
    max_payload_bytes: usize,
    max_snapshot_bytes: usize,
    max_push_batch: usize,
}

impl OfflineSyncPolicy {
    /// Builds a policy while enforcing the framework's hard ceilings.
    pub const fn new(
        max_records: usize,
        max_pending_mutations: usize,
        max_conflicts: usize,
        max_payload_bytes: usize,
        max_snapshot_bytes: usize,
        max_push_batch: usize,
    ) -> Result<Self, OfflineSyncError> {
        if max_records == 0
            || max_records > MAX_OFFLINE_RECORDS
            || max_pending_mutations == 0
            || max_pending_mutations > MAX_OFFLINE_PENDING_MUTATIONS
            || max_conflicts == 0
            || max_conflicts > MAX_OFFLINE_CONFLICTS
            || max_payload_bytes == 0
            || max_payload_bytes > MAX_OFFLINE_PAYLOAD_BYTES
            || max_snapshot_bytes == 0
            || max_snapshot_bytes > MAX_OFFLINE_SNAPSHOT_BYTES
            || max_push_batch == 0
            || max_push_batch > MAX_OFFLINE_PUSH_BATCH
        {
            return Err(OfflineSyncError::InvalidPolicy);
        }
        Ok(Self {
            max_records,
            max_pending_mutations,
            max_conflicts,
            max_payload_bytes,
            max_snapshot_bytes,
            max_push_batch,
        })
    }

    /// Returns the cached-record quota.
    pub const fn max_records(self) -> usize {
        self.max_records
    }

    /// Returns the pending-mutation quota.
    pub const fn max_pending_mutations(self) -> usize {
        self.max_pending_mutations
    }

    /// Returns the unresolved-conflict quota.
    pub const fn max_conflicts(self) -> usize {
        self.max_conflicts
    }

    /// Returns the per-payload encoded JSON ceiling.
    pub const fn max_payload_bytes(self) -> usize {
        self.max_payload_bytes
    }

    /// Returns the plaintext snapshot ceiling.
    pub const fn max_snapshot_bytes(self) -> usize {
        self.max_snapshot_bytes
    }

    /// Returns the per-request mutation ceiling.
    pub const fn max_push_batch(self) -> usize {
        self.max_push_batch
    }
}

impl Default for OfflineSyncPolicy {
    fn default() -> Self {
        Self {
            max_records: 10_000,
            max_pending_mutations: 500,
            max_conflicts: 500,
            max_payload_bytes: 128 * 1024,
            max_snapshot_bytes: 8 * 1024 * 1024,
            max_push_batch: 50,
        }
    }
}
