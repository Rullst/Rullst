use super::types::validate_record;
use super::{OfflineMutation, OfflineSyncError, OfflineSyncPolicy, ServerRecord};
use std::collections::HashSet;

pub(super) fn validate_server_page(
    records: &[ServerRecord],
    policy: OfflineSyncPolicy,
) -> Result<(), OfflineSyncError> {
    if records.len() > policy.max_records() {
        return Err(OfflineSyncError::RecordQuotaExceeded {
            maximum: policy.max_records(),
        });
    }
    let mut keys = HashSet::with_capacity(records.len());
    for record in records {
        validate_record(record, policy)?;
        if !keys.insert((record.key().collection(), record.key().entity_id())) {
            return Err(OfflineSyncError::DuplicateServerRecord);
        }
    }
    Ok(())
}

pub(super) fn ensure_matching_record(
    mutation: &OfflineMutation,
    record: &ServerRecord,
) -> Result<(), OfflineSyncError> {
    if mutation.entity() == record.key() {
        Ok(())
    } else {
        Err(OfflineSyncError::RecordKeyMismatch)
    }
}
