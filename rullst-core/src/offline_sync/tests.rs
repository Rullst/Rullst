#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::client_contract::{FailureCode, IdempotencyKey};
use serde_json::json;

mod orchestration;
mod recovery;

fn policy() -> OfflineSyncPolicy {
    OfflineSyncPolicy::new(8, 6, 6, 2_048, 32_768, 3).expect("valid test policy")
}

fn account(value: &str) -> OfflineAccountId {
    OfflineAccountId::new(value).expect("valid account")
}

fn entity(value: &str) -> OfflineEntityKey {
    OfflineEntityKey::new("lesson_attempts", value).expect("valid entity")
}

fn replay(value: &str) -> IdempotencyKey {
    IdempotencyKey::new(value).expect("valid replay key")
}

fn record(id: &str, revision: u64, score: u64) -> ServerRecord {
    ServerRecord::new(
        entity(id),
        revision,
        OfflineRecordValue::Upsert(json!({ "score": score })),
    )
    .expect("valid record")
}

fn upsert(id: &str, key: &str, base_revision: Option<u64>) -> OfflineMutation {
    OfflineMutation::upsert(
        replay(key),
        entity(id),
        base_revision,
        123,
        json!({ "score": 10 }),
    )
    .expect("valid mutation")
}

#[test]
fn identifiers_payloads_and_policy_are_bounded() {
    assert!(OfflineAccountId::new("").is_err());
    assert!(OfflineEntityKey::new("bad/name", "1").is_err());
    assert!(SyncCursor::new("cursor with spaces").is_err());
    assert!(OfflineSyncPolicy::new(0, 1, 1, 1, 1, 1).is_err());
    assert!(ServerRecord::new(entity("1"), 0, OfflineRecordValue::Deleted).is_err());

    let payload = json!({ "large": "x".repeat(2_100) });
    let mutation = OfflineMutation::upsert(replay("large-key"), entity("1"), None, 0, payload)
        .expect("below the hard constructor bound");
    let mut state = OfflineSyncState::new(account("user-1"));
    assert!(matches!(
        state.queue(policy(), mutation),
        Err(OfflineSyncError::PayloadTooLarge { .. })
    ));
}

#[test]
fn pending_queue_is_fifo_bounded_and_replay_unique() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(policy(), upsert("a", "mutation-a", None))
        .expect("first mutation");
    state
        .queue(policy(), upsert("b", "mutation-b", None))
        .expect("second mutation");
    assert!(matches!(
        state.queue(policy(), upsert("c", "mutation-a", None)),
        Err(OfflineSyncError::DuplicateIdempotencyKey)
    ));

    let batch = state.push_batch(policy(), 1).expect("bounded batch");
    assert_eq!(batch.mutations().len(), 1);
    assert_eq!(batch.mutations()[0].entity(), &entity("a"));
    assert!(state.push_batch(policy(), 0).is_err());
    assert!(state.push_batch(policy(), 4).is_err());
}

#[test]
fn state_and_server_batches_honor_global_bounds_atomically() {
    let tiny_snapshot = OfflineSyncPolicy::new(8, 6, 6, 2_048, 64, 3).expect("valid small policy");
    let mut state = OfflineSyncState::new(account("user-1"));
    assert!(matches!(
        state.queue(tiny_snapshot, upsert("a", "mutation-a", None)),
        Err(OfflineSyncError::SnapshotTooLarge { maximum: 64 })
    ));
    assert!(state.pending().is_empty());

    state
        .queue(policy(), upsert("a", "mutation-a", None))
        .expect("queued mutation");
    let before = state.clone();
    let outcomes = (0..4)
        .map(|index| MutationOutcome::Rejected {
            idempotency_key: replay(&format!("server-{index}")),
            code: FailureCode::new("request.rejected").expect("code"),
            retryable: true,
        })
        .collect();
    assert!(matches!(
        state.apply_push(policy(), SyncPushResult::new(None, outcomes), 1_000),
        Err(OfflineSyncError::InvalidBatchLimit)
    ));
    assert_eq!(state, before);
}

#[test]
fn push_decisions_are_atomic_and_conflicts_require_explicit_resolution() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(policy(), upsert("a", "mutation-a", None))
        .expect("queue mutation");
    state
        .apply_push(
            policy(),
            SyncPushResult::new(
                Some(SyncCursor::new("cursor-1").expect("cursor")),
                vec![MutationOutcome::Applied {
                    idempotency_key: replay("mutation-a"),
                    record: record("a", 1, 10),
                }],
            ),
            1_000,
        )
        .expect("apply acknowledgement");
    assert!(state.pending().is_empty());
    assert_eq!(
        state.record(&entity("a")).map(ServerRecord::revision),
        Some(1)
    );

    state
        .queue(policy(), upsert("a", "mutation-b", Some(1)))
        .expect("queue update");
    state
        .apply_push(
            policy(),
            SyncPushResult::new(
                Some(SyncCursor::new("cursor-2").expect("cursor")),
                vec![MutationOutcome::Conflict {
                    idempotency_key: replay("mutation-b"),
                    server_record: record("a", 2, 20),
                    code: FailureCode::new("lesson.concurrent_edit").expect("code"),
                }],
            ),
            1_001,
        )
        .expect("isolate conflict");
    assert!(state.pending().is_empty());
    assert_eq!(state.conflicts().len(), 1);
    assert!(matches!(
        state.conflicts()[0].reason(),
        OfflineConflictReason::ServerConflict(_)
    ));

    state
        .resolve_conflict(
            policy(),
            &replay("mutation-b"),
            ConflictResolution::RetryAgainstServer {
                idempotency_key: replay("mutation-c"),
            },
        )
        .expect("explicit retry");
    assert!(state.conflicts().is_empty());
    assert_eq!(state.pending()[0].base_revision(), Some(2));
    assert_eq!(state.pending()[0].idempotency_key().as_str(), "mutation-c");

    let before = state.clone();
    let result = SyncPushResult::new(
        None,
        vec![
            MutationOutcome::Rejected {
                idempotency_key: replay("mutation-c"),
                code: FailureCode::new("network.retry").expect("code"),
                retryable: true,
            },
            MutationOutcome::Rejected {
                idempotency_key: replay("mutation-c"),
                code: FailureCode::new("network.retry").expect("code"),
                retryable: true,
            },
        ],
    );
    assert!(matches!(
        state.apply_push(policy(), result, 1_002),
        Err(OfflineSyncError::DuplicateOutcome)
    ));
    assert_eq!(state, before);
}

#[test]
fn incremental_pull_never_overwrites_a_divergent_local_edit_silently() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .replace_server_snapshot(
            policy(),
            vec![record("a", 1, 10)],
            SyncCursor::new("cursor-1").expect("cursor"),
            1_000,
        )
        .expect("baseline snapshot");
    state
        .queue(policy(), upsert("a", "mutation-a", Some(1)))
        .expect("local edit");
    state
        .apply_pull(
            policy(),
            SyncPullPage::new(
                SyncCursor::new("cursor-2").expect("cursor"),
                vec![record("a", 2, 20)],
                false,
                false,
            ),
            1_001,
        )
        .expect("pull conflict");

    assert!(state.pending().is_empty());
    assert_eq!(state.conflicts().len(), 1);
    assert!(matches!(
        state.conflicts()[0].reason(),
        OfflineConflictReason::ConcurrentServerChange
    ));
    assert_eq!(
        state.record(&entity("a")).map(ServerRecord::revision),
        Some(2)
    );
}

#[test]
fn full_resync_signal_and_revision_regression_leave_state_unchanged() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .replace_server_snapshot(
            policy(),
            vec![record("a", 2, 20)],
            SyncCursor::new("cursor-2").expect("cursor"),
            2_000,
        )
        .expect("baseline snapshot");
    let before = state.clone();
    assert!(matches!(
        state.apply_pull(
            policy(),
            SyncPullPage::new(
                SyncCursor::new("cursor-old").expect("cursor"),
                vec![],
                false,
                true,
            ),
            2_001,
        ),
        Err(OfflineSyncError::FullResyncRequired)
    ));
    assert_eq!(state, before);

    assert!(matches!(
        state.apply_pull(
            policy(),
            SyncPullPage::new(
                SyncCursor::new("cursor-3").expect("cursor"),
                vec![record("a", 1, 5)],
                false,
                false,
            ),
            2_001,
        ),
        Err(OfflineSyncError::RevisionRegressed)
    ));
    assert_eq!(state, before);
}

#[test]
fn complete_resync_preserves_new_work_and_isolates_stale_work() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(policy(), upsert("a", "mutation-a", Some(1)))
        .expect("stale edit");
    state
        .queue(policy(), upsert("new", "mutation-new", None))
        .expect("new entity");
    state
        .replace_server_snapshot(
            policy(),
            vec![record("a", 2, 20)],
            SyncCursor::new("cursor-full").expect("cursor"),
            5_000,
        )
        .expect("full resync");
    assert_eq!(state.pending().len(), 1);
    assert_eq!(state.pending()[0].entity(), &entity("new"));
    assert_eq!(state.conflicts().len(), 1);
    assert!(matches!(
        state.conflicts()[0].reason(),
        OfflineConflictReason::FullResyncDivergence
    ));
}

#[test]
fn recovery_preserves_local_work_and_erasure_clears_all_account_state() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .replace_server_snapshot(
            policy(),
            vec![record("a", 1, 10)],
            SyncCursor::new("cursor-1").expect("cursor"),
            1_000,
        )
        .expect("snapshot");
    state
        .queue(policy(), upsert("new", "mutation-new", None))
        .expect("pending");
    let recovered = state.recover_server_cache();
    assert_eq!(recovered.records_cleared, 1);
    assert_eq!(recovered.pending_preserved, 1);
    assert!(state.cursor().is_none());

    let erased = state.erase();
    assert_eq!(erased.pending_erased, 1);
    assert!(state.records().is_empty());
    assert!(state.pending().is_empty());
    assert!(state.conflicts().is_empty());
}

#[test]
fn encrypted_snapshot_is_randomized_account_bound_and_tamper_evident() {
    let policy = policy();
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(policy, upsert("a", "mutation-a", None))
        .expect("pending mutation");
    let key = [7_u8; 32];
    let cipher = OfflineSnapshotCipher::new("device-key-1", key).expect("cipher");
    let first = cipher.seal(policy, &state).expect("sealed snapshot");
    let second = cipher.seal(policy, &state).expect("new nonce");
    assert_ne!(first, second);
    assert_eq!(
        cipher
            .open(policy, &account("user-1"), &first)
            .expect("round trip"),
        state
    );
    assert!(matches!(
        cipher.open(policy, &account("user-2"), &first),
        Err(OfflineSyncError::AuthenticationFailed)
    ));

    let mut tampered = first.clone();
    let final_byte = tampered.last_mut().expect("ciphertext byte");
    *final_byte ^= 1;
    assert!(matches!(
        cipher.open(policy, &account("user-1"), &tampered),
        Err(OfflineSyncError::AuthenticationFailed)
    ));
    let other_id = OfflineSnapshotCipher::new("device-key-2", key).expect("cipher");
    assert!(matches!(
        other_id.open(policy, &account("user-1"), &first),
        Err(OfflineSyncError::SnapshotKeyIdMismatch)
    ));
    assert!(!format!("{cipher:?}").contains("070707"));
    assert!(format!("{cipher:?}").contains("[REDACTED]"));
    let state_debug = format!("{state:?}");
    assert!(!state_debug.contains("user-1"));
    assert!(!state_debug.contains("score"));
}

#[test]
fn decoded_snapshots_revalidate_schema_quotas_and_replay_uniqueness() {
    let policy = policy();
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(policy, upsert("a", "mutation-a", None))
        .expect("pending mutation");
    let mut wire = serde_json::to_value(&state).expect("state JSON");
    wire["schema_version"] = json!(99);
    let bytes = serde_json::to_vec(&wire).expect("wire JSON");
    assert!(matches!(
        OfflineSyncState::decode(policy, &bytes),
        Err(OfflineSyncError::UnsupportedSnapshotVersion(99))
    ));

    wire["schema_version"] = json!(OFFLINE_SNAPSHOT_VERSION);
    let duplicate = wire["pending"][0].clone();
    wire["pending"]
        .as_array_mut()
        .expect("array")
        .push(duplicate);
    let bytes = serde_json::to_vec(&wire).expect("wire JSON");
    assert!(matches!(
        OfflineSyncState::decode(policy, &bytes),
        Err(OfflineSyncError::DuplicateIdempotencyKey)
    ));
}

#[test]
fn server_time_and_page_shape_are_fail_closed() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .replace_server_snapshot(
            policy(),
            vec![],
            SyncCursor::new("cursor-1").expect("cursor"),
            10,
        )
        .expect("initial time");
    let before = state.clone();
    assert!(matches!(
        state.apply_pull(
            policy(),
            SyncPullPage::new(
                SyncCursor::new("cursor-2").expect("cursor"),
                vec![],
                false,
                false,
            ),
            9,
        ),
        Err(OfflineSyncError::ServerTimeRegressed)
    ));
    assert_eq!(state, before);

    assert!(matches!(
        state.apply_pull(
            policy(),
            SyncPullPage::new(
                SyncCursor::new("cursor-2").expect("cursor"),
                vec![record("a", 1, 10), record("a", 2, 20)],
                false,
                false,
            ),
            11,
        ),
        Err(OfflineSyncError::DuplicateServerRecord)
    ));
    assert_eq!(state, before);
}
