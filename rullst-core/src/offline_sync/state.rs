use super::types::{validate_mutation, validate_record};
use super::validation::{ensure_matching_record, validate_server_page};
use super::{
    ConflictResolution, ErasureSummary, MutationOutcome, OFFLINE_SNAPSHOT_VERSION,
    OfflineAccountId, OfflineConflict, OfflineConflictReason, OfflineEntityKey, OfflineMutation,
    OfflineSyncError, OfflineSyncPolicy, RecoverySummary, ServerRecord, SyncCursor, SyncPullPage,
    SyncPushBatch, SyncPushResult,
};
use crate::client_contract::IdempotencyKey;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Versioned bounded state for one authenticated application account.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct OfflineSyncState {
    pub(super) schema_version: u16,
    pub(super) account_id: OfflineAccountId,
    pub(super) cursor: Option<SyncCursor>,
    pub(super) records: Vec<ServerRecord>,
    pub(super) pending: Vec<OfflineMutation>,
    pub(super) conflicts: Vec<OfflineConflict>,
    pub(super) last_server_epoch_ms: Option<u64>,
}

impl std::fmt::Debug for OfflineSyncState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OfflineSyncState")
            .field("schema_version", &self.schema_version)
            .field("account_id", &"[REDACTED]")
            .field("cursor_present", &self.cursor.is_some())
            .field("record_count", &self.records.len())
            .field("pending_count", &self.pending.len())
            .field("conflict_count", &self.conflicts.len())
            .field("server_time_present", &self.last_server_epoch_ms.is_some())
            .finish()
    }
}

impl OfflineSyncState {
    /// Creates an empty account-bound state using the current schema.
    pub const fn new(account_id: OfflineAccountId) -> Self {
        Self {
            schema_version: OFFLINE_SNAPSHOT_VERSION,
            account_id,
            cursor: None,
            records: Vec::new(),
            pending: Vec::new(),
            conflicts: Vec::new(),
            last_server_epoch_ms: None,
        }
    }

    /// Returns the account cryptographically bound by snapshot encryption.
    pub const fn account_id(&self) -> &OfflineAccountId {
        &self.account_id
    }

    /// Returns the last fully applied server cursor.
    pub const fn cursor(&self) -> Option<&SyncCursor> {
        self.cursor.as_ref()
    }

    /// Returns cached authoritative records, including deletion tombstones.
    pub fn records(&self) -> &[ServerRecord] {
        &self.records
    }

    /// Returns mutations still eligible for FIFO push.
    pub fn pending(&self) -> &[OfflineMutation] {
        &self.pending
    }

    /// Returns mutations isolated from automatic replay.
    pub fn conflicts(&self) -> &[OfflineConflict] {
        &self.conflicts
    }

    /// Returns the latest accepted server-authored time.
    pub const fn last_server_epoch_ms(&self) -> Option<u64> {
        self.last_server_epoch_ms
    }

    /// Returns one cached server record by exact application key.
    pub fn record(&self, key: &OfflineEntityKey) -> Option<&ServerRecord> {
        self.records.iter().find(|record| record.key() == key)
    }

    /// Adds one bounded mutation while preventing replay-key reuse.
    pub fn queue(
        &mut self,
        policy: OfflineSyncPolicy,
        mutation: OfflineMutation,
    ) -> Result<(), OfflineSyncError> {
        validate_mutation(&mutation, policy)?;
        self.ensure_unique_idempotency(mutation.idempotency_key())?;
        if self.pending.len() >= policy.max_pending_mutations() {
            return Err(OfflineSyncError::PendingQuotaExceeded {
                maximum: policy.max_pending_mutations(),
            });
        }
        self.pending.push(mutation);
        if let Err(error) = self.validate_encoded_size(policy) {
            self.pending.pop();
            return Err(error);
        }
        Ok(())
    }

    /// Copies the next FIFO mutations into a bounded transport batch.
    pub fn push_batch(
        &self,
        policy: OfflineSyncPolicy,
        requested_limit: usize,
    ) -> Result<SyncPushBatch, OfflineSyncError> {
        if requested_limit == 0 || requested_limit > policy.max_push_batch() {
            return Err(OfflineSyncError::InvalidBatchLimit);
        }
        Ok(SyncPushBatch {
            cursor: self.cursor.clone(),
            mutations: self.pending.iter().take(requested_limit).cloned().collect(),
        })
    }

    /// Atomically applies server decisions for queued mutations.
    pub fn apply_push(
        &mut self,
        policy: OfflineSyncPolicy,
        result: SyncPushResult,
        server_epoch_ms: u64,
    ) -> Result<(), OfflineSyncError> {
        if result.outcomes.len() > policy.max_push_batch() {
            return Err(OfflineSyncError::InvalidBatchLimit);
        }
        {
            let mut seen = HashSet::with_capacity(result.outcomes.len());
            for outcome in &result.outcomes {
                if !seen.insert(outcome.idempotency_key().as_str()) {
                    return Err(OfflineSyncError::DuplicateOutcome);
                }
            }
        }
        let mut next = self.clone();
        next.accept_server_time(server_epoch_ms)?;
        let mut record_positions = next.record_positions();
        for outcome in result.outcomes {
            next.apply_outcome(policy, outcome, &mut record_positions)?;
        }
        if let Some(cursor) = result.cursor {
            next.cursor = Some(cursor);
        }
        next.validate_encoded_size(policy)?;
        *self = next;
        Ok(())
    }

    /// Atomically applies one incremental server page.
    pub fn apply_pull(
        &mut self,
        policy: OfflineSyncPolicy,
        page: SyncPullPage,
        server_epoch_ms: u64,
    ) -> Result<(), OfflineSyncError> {
        if page.requires_full_resync {
            return Err(OfflineSyncError::FullResyncRequired);
        }
        validate_server_page(&page.changes, policy)?;
        let mut next = self.clone();
        next.accept_server_time(server_epoch_ms)?;
        next.isolate_divergent_pending_for_page(policy, &page.changes)?;
        let mut record_positions = next.record_positions();
        for record in page.changes {
            next.upsert_server_record(policy, record, &mut record_positions)?;
        }
        next.cursor = Some(page.cursor);
        next.validate_encoded_size(policy)?;
        *self = next;
        Ok(())
    }

    /// Replaces derived server cache after an explicit complete resync.
    ///
    /// Pending local proposals are preserved only when their base revision still
    /// matches the complete authoritative snapshot; divergent proposals become
    /// conflicts and stop automatic replay.
    pub fn replace_server_snapshot(
        &mut self,
        policy: OfflineSyncPolicy,
        records: Vec<ServerRecord>,
        cursor: SyncCursor,
        server_epoch_ms: u64,
    ) -> Result<(), OfflineSyncError> {
        validate_server_page(&records, policy)?;
        let mut next = self.clone();
        next.accept_server_time(server_epoch_ms)?;
        next.records = records;
        let revisions = next
            .records
            .iter()
            .map(|record| (record.key().clone(), record.revision()))
            .collect::<HashMap<_, _>>();
        for conflict in &mut next.conflicts {
            conflict.authoritative_revision = revisions.get(conflict.mutation.entity()).copied();
        }

        let pending = std::mem::take(&mut next.pending);
        for mutation in pending {
            let server_record = next.record(mutation.entity()).cloned();
            let authoritative_revision = server_record.as_ref().map(ServerRecord::revision);
            if mutation.base_revision() == authoritative_revision {
                next.pending.push(mutation);
            } else {
                next.push_conflict(
                    policy,
                    OfflineConflict {
                        mutation,
                        authoritative_revision,
                        reason: OfflineConflictReason::FullResyncDivergence,
                    },
                )?;
            }
        }
        next.cursor = Some(cursor);
        next.validate_encoded_size(policy)?;
        *self = next;
        Ok(())
    }

    /// Resolves one isolated conflict without an implicit client-wins policy.
    pub fn resolve_conflict(
        &mut self,
        policy: OfflineSyncPolicy,
        decided_key: &IdempotencyKey,
        resolution: ConflictResolution,
    ) -> Result<(), OfflineSyncError> {
        let mut next = self.clone();
        let position = next
            .conflicts
            .iter()
            .position(|conflict| conflict.mutation.idempotency_key() == decided_key)
            .ok_or(OfflineSyncError::UnknownConflict)?;
        let conflict = next.conflicts[position].clone();
        match resolution {
            ConflictResolution::AcceptServer => {
                next.conflicts.remove(position);
            }
            ConflictResolution::RetryAgainstServer { idempotency_key } => {
                next.ensure_unique_idempotency(&idempotency_key)?;
                if next.pending.len() >= policy.max_pending_mutations() {
                    return Err(OfflineSyncError::PendingQuotaExceeded {
                        maximum: policy.max_pending_mutations(),
                    });
                }
                let base_revision = next
                    .record(conflict.mutation.entity())
                    .map(ServerRecord::revision);
                if base_revision.is_none() && next.cursor.is_none() {
                    return Err(OfflineSyncError::FullResyncRequired);
                }
                let retry = conflict
                    .mutation
                    .replay_from(idempotency_key, base_revision);
                validate_mutation(&retry, policy)?;
                next.conflicts.remove(position);
                next.pending.push(retry);
            }
        }
        next.validate_encoded_size(policy)?;
        *self = next;
        Ok(())
    }

    /// Drops derived server cache before a clean pull while preserving local work.
    pub fn recover_server_cache(&mut self) -> RecoverySummary {
        let summary = RecoverySummary {
            records_cleared: self.records.len(),
            pending_preserved: self.pending.len(),
            conflicts_preserved: self.conflicts.len(),
        };
        self.records.clear();
        self.cursor = None;
        self.last_server_epoch_ms = None;
        for conflict in &mut self.conflicts {
            conflict.authoritative_revision = None;
        }
        summary
    }

    /// Logically erases all account data held by this state object.
    ///
    /// Callers must also delete every persisted encrypted snapshot and remove
    /// its key from platform secure storage. Rust cannot erase prior copies.
    pub fn erase(&mut self) -> ErasureSummary {
        let summary = ErasureSummary {
            records_erased: self.records.len(),
            pending_erased: self.pending.len(),
            conflicts_erased: self.conflicts.len(),
        };
        self.records.clear();
        self.pending.clear();
        self.conflicts.clear();
        self.cursor = None;
        self.last_server_epoch_ms = None;
        summary
    }

    fn apply_outcome(
        &mut self,
        policy: OfflineSyncPolicy,
        outcome: MutationOutcome,
        record_positions: &mut HashMap<OfflineEntityKey, usize>,
    ) -> Result<(), OfflineSyncError> {
        let position = self
            .pending
            .iter()
            .position(|mutation| mutation.idempotency_key() == outcome.idempotency_key())
            .ok_or(OfflineSyncError::UnknownMutation)?;
        let mutation = self.pending[position].clone();
        match outcome {
            MutationOutcome::Applied { record, .. } => {
                ensure_matching_record(&mutation, &record)?;
                validate_record(&record, policy)?;
                self.upsert_server_record(policy, record, record_positions)?;
                self.pending.remove(position);
            }
            MutationOutcome::Conflict {
                server_record,
                code,
                ..
            } => {
                ensure_matching_record(&mutation, &server_record)?;
                validate_record(&server_record, policy)?;
                self.push_conflict(
                    policy,
                    OfflineConflict {
                        mutation,
                        authoritative_revision: Some(server_record.revision()),
                        reason: OfflineConflictReason::ServerConflict(code),
                    },
                )?;
                self.upsert_server_record(policy, server_record, record_positions)?;
                self.pending.remove(position);
            }
            MutationOutcome::Rejected {
                code, retryable, ..
            } => {
                if !retryable {
                    let server_record = self.record(mutation.entity()).cloned();
                    self.push_conflict(
                        policy,
                        OfflineConflict {
                            mutation,
                            authoritative_revision: server_record
                                .as_ref()
                                .map(ServerRecord::revision),
                            reason: OfflineConflictReason::PermanentRejection(code),
                        },
                    )?;
                    self.pending.remove(position);
                }
            }
        }
        Ok(())
    }

    fn isolate_divergent_pending_for_page(
        &mut self,
        policy: OfflineSyncPolicy,
        records: &[ServerRecord],
    ) -> Result<(), OfflineSyncError> {
        let revisions = records
            .iter()
            .map(|record| (record.key().clone(), record.revision()))
            .collect::<HashMap<_, _>>();
        let divergent = self
            .pending
            .iter()
            .filter(|mutation| {
                revisions
                    .get(mutation.entity())
                    .is_some_and(|revision| mutation.base_revision() != Some(*revision))
            })
            .count();
        if self.conflicts.len().saturating_add(divergent) > policy.max_conflicts() {
            return Err(OfflineSyncError::ConflictQuotaExceeded {
                maximum: policy.max_conflicts(),
            });
        }
        let pending = std::mem::take(&mut self.pending);
        for mutation in pending {
            if let Some(revision) = revisions
                .get(mutation.entity())
                .filter(|revision| mutation.base_revision() != Some(**revision))
            {
                self.conflicts.push(OfflineConflict {
                    mutation,
                    authoritative_revision: Some(*revision),
                    reason: OfflineConflictReason::ConcurrentServerChange,
                });
            } else {
                self.pending.push(mutation);
            }
        }
        Ok(())
    }

    fn upsert_server_record(
        &mut self,
        policy: OfflineSyncPolicy,
        record: ServerRecord,
        positions: &mut HashMap<OfflineEntityKey, usize>,
    ) -> Result<(), OfflineSyncError> {
        validate_record(&record, policy)?;
        if let Some(position) = positions.get(record.key()).copied() {
            let existing = &self.records[position];
            if record.revision() < existing.revision() {
                return Err(OfflineSyncError::RevisionRegressed);
            }
            if record.revision() == existing.revision() && *existing != record {
                return Err(OfflineSyncError::RevisionCollision);
            }
            self.records[position] = record.clone();
            self.refresh_conflict_revisions(&record);
            return Ok(());
        }
        if self.records.len() >= policy.max_records() {
            return Err(OfflineSyncError::RecordQuotaExceeded {
                maximum: policy.max_records(),
            });
        }
        let position = self.records.len();
        self.records.push(record.clone());
        positions.insert(record.key().clone(), position);
        self.refresh_conflict_revisions(&record);
        Ok(())
    }

    fn record_positions(&self) -> HashMap<OfflineEntityKey, usize> {
        self.records
            .iter()
            .enumerate()
            .map(|(position, record)| (record.key().clone(), position))
            .collect()
    }

    fn refresh_conflict_revisions(&mut self, record: &ServerRecord) {
        for conflict in &mut self.conflicts {
            if conflict.mutation.entity() == record.key() {
                conflict.authoritative_revision = Some(record.revision());
            }
        }
    }

    fn push_conflict(
        &mut self,
        policy: OfflineSyncPolicy,
        conflict: OfflineConflict,
    ) -> Result<(), OfflineSyncError> {
        if self.conflicts.len() >= policy.max_conflicts() {
            return Err(OfflineSyncError::ConflictQuotaExceeded {
                maximum: policy.max_conflicts(),
            });
        }
        self.conflicts.push(conflict);
        Ok(())
    }

    fn ensure_unique_idempotency(&self, key: &IdempotencyKey) -> Result<(), OfflineSyncError> {
        let exists = self
            .pending
            .iter()
            .any(|mutation| mutation.idempotency_key() == key)
            || self
                .conflicts
                .iter()
                .any(|conflict| conflict.mutation.idempotency_key() == key);
        if exists {
            Err(OfflineSyncError::DuplicateIdempotencyKey)
        } else {
            Ok(())
        }
    }

    fn accept_server_time(&mut self, server_epoch_ms: u64) -> Result<(), OfflineSyncError> {
        if self
            .last_server_epoch_ms
            .is_some_and(|last| server_epoch_ms < last)
        {
            return Err(OfflineSyncError::ServerTimeRegressed);
        }
        self.last_server_epoch_ms = Some(server_epoch_ms);
        Ok(())
    }
}
