#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::types::{validate_mutation, validate_record};
use super::*;
use crate::client_contract::{FailureCode, IdempotencyKey};
use serde_json::json;

fn policy_with_quotas(records: usize, pending: usize, conflicts: usize) -> OfflineSyncPolicy {
    OfflineSyncPolicy::new(records, pending, conflicts, 2_048, 32_768, 3)
        .expect("valid constrained policy")
}

fn account() -> OfflineAccountId {
    OfflineAccountId::new("account-1").expect("valid account")
}

fn entity(id: &str) -> OfflineEntityKey {
    OfflineEntityKey::new("lessons", id).expect("valid entity")
}

fn replay(id: &str) -> IdempotencyKey {
    IdempotencyKey::new(id).expect("valid replay key")
}

fn upsert(id: &str, replay_key: &str, base_revision: Option<u64>) -> OfflineMutation {
    OfflineMutation::upsert(
        replay(replay_key),
        entity(id),
        base_revision,
        1_234,
        json!({ "answer": 42 }),
    )
    .expect("valid mutation")
}

fn record(id: &str, revision: u64, answer: u64) -> ServerRecord {
    ServerRecord::new(
        entity(id),
        revision,
        OfflineRecordValue::Upsert(json!({ "answer": answer })),
    )
    .expect("valid record")
}

fn permanent_rejection(replay_key: &str) -> MutationOutcome {
    MutationOutcome::Rejected {
        idempotency_key: replay(replay_key),
        code: FailureCode::new("lesson.invalid").expect("valid failure code"),
        retryable: false,
    }
}

#[test]
fn wire_types_validate_tokens_redact_payloads_and_expose_metadata() {
    let account = account();
    assert_eq!(account.as_str(), "account-1");
    let key = entity("lesson-1");
    assert_eq!(key.collection(), "lessons");
    assert_eq!(key.entity_id(), "lesson-1");
    assert!(matches!(
        OfflineEntityKey::new("lessons", "contains spaces"),
        Err(OfflineSyncError::InvalidEntityId)
    ));

    let cursor: SyncCursor = serde_json::from_str("\"cursor:2\"").expect("valid cursor wire");
    assert_eq!(cursor.as_str(), "cursor:2");
    assert!(serde_json::from_str::<SyncCursor>("\"bad cursor\"").is_err());

    let value = OfflineRecordValue::Upsert(json!({ "secret": "never-log-this" }));
    assert_eq!(format!("{value:?}"), "Upsert([REDACTED])");
    assert_eq!(format!("{:?}", OfflineRecordValue::Deleted), "Deleted");
    let server_record = ServerRecord::new(key.clone(), 7, value).expect("valid record");
    assert!(matches!(
        server_record.value(),
        OfflineRecordValue::Upsert(_)
    ));

    let deletion = OfflineMutation::delete(replay("delete-1"), key, Some(7), 9_876);
    assert_eq!(deletion.queued_epoch_ms(), 9_876);
    assert!(matches!(deletion.operation(), OfflineMutationKind::Delete));
    assert_eq!(format!("{:?}", deletion.operation()), "Delete");
    assert_eq!(
        format!(
            "{:?}",
            OfflineMutationKind::Upsert(json!({ "secret": "never-log-this" }))
        ),
        "Upsert([REDACTED])"
    );

    let page = SyncPullPage::new(cursor, vec![server_record], true, true);
    assert!(page.has_more());
    assert!(page.requires_full_resync());
}

#[test]
fn constructor_and_decoded_values_obey_payload_and_revision_bounds() {
    let oversized = json!({ "data": "x".repeat(512 * 1_024) });
    assert!(matches!(
        ServerRecord::new(entity("large"), 1, OfflineRecordValue::Upsert(oversized)),
        Err(OfflineSyncError::PayloadTooLarge { .. })
    ));

    let invalid_record: ServerRecord = serde_json::from_value(json!({
        "key": { "collection": "lessons", "entity_id": "one" },
        "revision": 0,
        "value": { "kind": "deleted" }
    }))
    .expect("constructor-bypassing wire record");
    assert!(matches!(
        validate_record(&invalid_record, OfflineSyncPolicy::default()),
        Err(OfflineSyncError::InvalidRevision)
    ));

    let invalid_mutation: OfflineMutation = serde_json::from_value(json!({
        "idempotency_key": "invalid-base",
        "entity": { "collection": "lessons", "entity_id": "one" },
        "base_revision": 0,
        "queued_epoch_ms": 0,
        "operation": { "kind": "delete" }
    }))
    .expect("constructor-bypassing wire mutation");
    assert!(matches!(
        validate_mutation(&invalid_mutation, OfflineSyncPolicy::default()),
        Err(OfflineSyncError::InvalidRevision)
    ));
}

#[test]
fn queue_and_conflict_quotas_fail_atomically_without_losing_work() {
    let policy = policy_with_quotas(4, 1, 1);
    let mut state = OfflineSyncState::new(account());
    state
        .queue(policy, upsert("one", "mutation-1", None))
        .expect("first queued mutation");
    assert!(matches!(
        state.queue(policy, upsert("two", "mutation-2", None)),
        Err(OfflineSyncError::PendingQuotaExceeded { maximum: 1 })
    ));
    assert_eq!(state.pending().len(), 1);

    state
        .apply_push(
            policy,
            SyncPushResult::new(None, vec![permanent_rejection("mutation-1")]),
            10,
        )
        .expect("first permanent rejection");
    assert_eq!(state.conflicts().len(), 1);
    assert!(matches!(
        state.conflicts()[0].reason(),
        OfflineConflictReason::PermanentRejection(_)
    ));

    state
        .queue(policy, upsert("two", "mutation-2", None))
        .expect("queue remains available after isolation");
    let before = state.clone();
    assert!(matches!(
        state.apply_push(
            policy,
            SyncPushResult::new(None, vec![permanent_rejection("mutation-2")]),
            11,
        ),
        Err(OfflineSyncError::ConflictQuotaExceeded { maximum: 1 })
    ));
    assert_eq!(state, before);

    assert!(matches!(
        state.resolve_conflict(
            policy,
            &replay("mutation-1"),
            ConflictResolution::RetryAgainstServer {
                idempotency_key: replay("mutation-3"),
            },
        ),
        Err(OfflineSyncError::PendingQuotaExceeded { maximum: 1 })
    ));
    assert_eq!(state, before);

    state
        .resolve_conflict(
            policy,
            &replay("mutation-1"),
            ConflictResolution::AcceptServer,
        )
        .expect("accept authoritative state");
    assert!(state.conflicts().is_empty());
    assert_eq!(state.pending(), before.pending());
}

#[test]
fn record_quota_and_revision_collision_leave_authoritative_cache_unchanged() {
    let policy = policy_with_quotas(1, 2, 2);
    let mut state = OfflineSyncState::new(account());
    state
        .replace_server_snapshot(
            policy,
            vec![record("one", 1, 10)],
            SyncCursor::new("cursor-1").expect("cursor"),
            100,
        )
        .expect("initial cache");
    assert_eq!(state.last_server_epoch_ms(), Some(100));

    let before = state.clone();
    assert!(matches!(
        state.apply_pull(
            policy,
            SyncPullPage::new(
                SyncCursor::new("cursor-2").expect("cursor"),
                vec![record("two", 1, 20)],
                false,
                false,
            ),
            101,
        ),
        Err(OfflineSyncError::RecordQuotaExceeded { maximum: 1 })
    ));
    assert_eq!(state, before);

    assert!(matches!(
        state.apply_pull(
            policy,
            SyncPullPage::new(
                SyncCursor::new("cursor-2").expect("cursor"),
                vec![record("one", 1, 99)],
                false,
                false,
            ),
            101,
        ),
        Err(OfflineSyncError::RevisionCollision)
    ));
    assert_eq!(state, before);
}

#[test]
fn cursor_propagates_into_push_batches_after_an_incremental_pull() {
    let policy = policy_with_quotas(2, 2, 2);
    let mut state = OfflineSyncState::new(account());
    state
        .apply_pull(
            policy,
            SyncPullPage::new(
                SyncCursor::new("cursor-current").expect("cursor"),
                vec![],
                false,
                false,
            ),
            700,
        )
        .expect("empty authoritative page");
    state
        .queue(policy, upsert("one", "mutation-1", None))
        .expect("queued mutation");
    let batch = state.push_batch(policy, 1).expect("push batch");
    assert_eq!(
        batch.cursor().map(SyncCursor::as_str),
        Some("cursor-current")
    );
    assert_eq!(state.account_id().as_str(), "account-1");
}
