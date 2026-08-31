use super::json_limit::{BoundedJsonError, encode_bounded, validate_bounded};
use super::types::validate_mutation;
use super::validation::validate_server_page;
use super::{OFFLINE_SNAPSHOT_VERSION, OfflineSyncError, OfflineSyncPolicy, OfflineSyncState};

impl OfflineSyncState {
    pub(super) fn encode(&self, policy: OfflineSyncPolicy) -> Result<Vec<u8>, OfflineSyncError> {
        self.validate_snapshot(policy)?;
        match encode_bounded(self, policy.max_snapshot_bytes()) {
            Ok(bytes) => Ok(bytes),
            Err(BoundedJsonError::LimitExceeded) => Err(OfflineSyncError::SnapshotTooLarge {
                maximum: policy.max_snapshot_bytes(),
            }),
            Err(BoundedJsonError::Encode(error)) => Err(OfflineSyncError::EncodeJson(error)),
        }
    }

    pub(super) fn validate_encoded_size(
        &self,
        policy: OfflineSyncPolicy,
    ) -> Result<(), OfflineSyncError> {
        self.validate_snapshot(policy)?;
        match validate_bounded(self, policy.max_snapshot_bytes()) {
            Ok(()) => Ok(()),
            Err(BoundedJsonError::LimitExceeded) => Err(OfflineSyncError::SnapshotTooLarge {
                maximum: policy.max_snapshot_bytes(),
            }),
            Err(BoundedJsonError::Encode(error)) => Err(OfflineSyncError::EncodeJson(error)),
        }
    }

    pub(super) fn decode(
        policy: OfflineSyncPolicy,
        bytes: &[u8],
    ) -> Result<Self, OfflineSyncError> {
        if bytes.len() > policy.max_snapshot_bytes() {
            return Err(OfflineSyncError::SnapshotTooLarge {
                maximum: policy.max_snapshot_bytes(),
            });
        }
        let state = serde_json::from_slice::<Self>(bytes).map_err(OfflineSyncError::InvalidJson)?;
        state.validate_snapshot(policy)?;
        Ok(state)
    }

    fn validate_snapshot(&self, policy: OfflineSyncPolicy) -> Result<(), OfflineSyncError> {
        if self.schema_version != OFFLINE_SNAPSHOT_VERSION {
            return Err(OfflineSyncError::UnsupportedSnapshotVersion(
                self.schema_version,
            ));
        }
        if self.records.len() > policy.max_records() {
            return Err(OfflineSyncError::RecordQuotaExceeded {
                maximum: policy.max_records(),
            });
        }
        if self.pending.len() > policy.max_pending_mutations() {
            return Err(OfflineSyncError::PendingQuotaExceeded {
                maximum: policy.max_pending_mutations(),
            });
        }
        if self.conflicts.len() > policy.max_conflicts() {
            return Err(OfflineSyncError::ConflictQuotaExceeded {
                maximum: policy.max_conflicts(),
            });
        }
        validate_server_page(&self.records, policy)?;
        for mutation in &self.pending {
            validate_mutation(mutation, policy)?;
        }
        for conflict in &self.conflicts {
            validate_mutation(&conflict.mutation, policy)?;
            if conflict.authoritative_revision == Some(0) {
                return Err(OfflineSyncError::InvalidRevision);
            }
        }
        let mut replay_keys = std::collections::HashSet::with_capacity(
            self.pending.len().saturating_add(self.conflicts.len()),
        );
        for mutation in &self.pending {
            if !replay_keys.insert(mutation.idempotency_key().as_str()) {
                return Err(OfflineSyncError::DuplicateIdempotencyKey);
            }
        }
        for conflict in &self.conflicts {
            if !replay_keys.insert(conflict.mutation.idempotency_key().as_str()) {
                return Err(OfflineSyncError::DuplicateIdempotencyKey);
            }
        }
        Ok(())
    }
}
