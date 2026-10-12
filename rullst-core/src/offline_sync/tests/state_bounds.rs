//! Exact batch and conflict bounds, replay-key checks and conflict revisions.

use super::*;

fn page(cursor: &str, changes: Vec<ServerRecord>) -> SyncPullPage {
    SyncPullPage::new(
        SyncCursor::new(cursor).expect("cursor"),
        changes,
        false,
        false,
    )
}

fn retryable_rejection(replay_key: &str) -> MutationOutcome {
    MutationOutcome::Rejected {
        idempotency_key: replay(replay_key),
        code: FailureCode::new("lesson.busy").expect("valid failure code"),
        retryable: true,
    }
}

#[test]
fn push_batches_and_results_may_fill_the_batch_limit() {
    let mut state = OfflineSyncState::new(account("user-1"));
    for (id, key) in [
        ("a", "mutation-a"),
        ("b", "mutation-b"),
        ("c", "mutation-c"),
    ] {
        state
            .queue(policy(), upsert(id, key, None))
            .expect("pending mutation");
    }
    let batch = state.push_batch(policy(), 3).expect("a full batch");
    assert_eq!(batch.mutations().len(), 3);

    let outcomes = ["mutation-a", "mutation-b", "mutation-c"]
        .into_iter()
        .map(retryable_rejection)
        .collect::<Vec<_>>();
    let result = SyncPushResult::new(Some(SyncCursor::new("cursor-9").expect("cursor")), outcomes);
    assert_eq!(result.cursor().map(SyncCursor::as_str), Some("cursor-9"));
    assert_eq!(result.outcomes().len(), 3);
    state
        .apply_push(policy(), result, 1_000)
        .expect("a full result");
    assert_eq!(state.cursor().map(SyncCursor::as_str), Some("cursor-9"));
    assert!(format!("{state:?}").contains("pending_count: 3"));
}

#[test]
fn divergence_may_fill_the_conflict_quota_and_matching_edits_do_not_count() {
    let one_conflict = OfflineSyncPolicy::new(8, 6, 1, 2_048, 32_768, 3).expect("policy");
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .replace_server_snapshot(
            one_conflict,
            vec![record("a", 1, 10), record("b", 1, 10)],
            SyncCursor::new("cursor-1").expect("cursor"),
            1_000,
        )
        .expect("baseline");
    state
        .queue(one_conflict, upsert("a", "mutation-a", Some(1)))
        .expect("edit of a");
    state
        .queue(one_conflict, upsert("b", "mutation-b", Some(1)))
        .expect("edit of b");

    // Unchanged records leave both edits pending.
    let unchanged = page("cursor-2", vec![record("a", 1, 10), record("b", 1, 10)]);
    assert!(!unchanged.requires_full_resync());
    state
        .apply_pull(one_conflict, unchanged, 1_001)
        .expect("no divergence");
    assert_eq!(state.pending().len(), 2);

    // One divergent edit exactly fills the quota of one conflict.
    state
        .apply_pull(
            one_conflict,
            page("cursor-3", vec![record("a", 2, 20)]),
            1_002,
        )
        .expect("one conflict");
    assert_eq!(state.conflicts().len(), 1);
    assert_eq!(state.pending().len(), 1);
}

#[test]
fn conflicts_follow_later_revisions_of_their_own_entity() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .replace_server_snapshot(
            policy(),
            vec![record("a", 1, 10), record("b", 1, 10)],
            SyncCursor::new("cursor-1").expect("cursor"),
            1_000,
        )
        .expect("baseline");
    state
        .queue(policy(), upsert("a", "mutation-a", Some(1)))
        .expect("edit of a");
    state
        .queue(policy(), upsert("b", "mutation-b", Some(1)))
        .expect("edit of b");
    state
        .apply_pull(
            policy(),
            page("cursor-2", vec![record("a", 2, 20), record("b", 2, 20)]),
            1_001,
        )
        .expect("two conflicts");
    state
        .apply_pull(policy(), page("cursor-3", vec![record("a", 3, 30)]), 1_002)
        .expect("newer record");

    let revision = |id: &str| {
        state
            .conflicts()
            .iter()
            .find(|conflict| conflict.mutation().entity() == &entity(id))
            .and_then(OfflineConflict::authoritative_revision)
    };
    assert_eq!(revision("a"), Some(3));
    assert_eq!(revision("b"), Some(2));
}

#[test]
fn a_reused_replay_key_is_reported_before_the_pending_quota() {
    let one_pending = OfflineSyncPolicy::new(8, 1, 6, 2_048, 32_768, 3).expect("policy");
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(one_pending, upsert("a", "mutation-a", None))
        .expect("first mutation");
    assert!(matches!(
        state.queue(one_pending, upsert("b", "mutation-a", None)),
        Err(OfflineSyncError::DuplicateIdempotencyKey)
    ));
}

#[test]
fn a_conflict_without_a_cached_record_may_be_retried_after_a_pull() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .replace_server_snapshot(
            policy(),
            Vec::new(),
            SyncCursor::new("cursor-1").expect("cursor"),
            1_000,
        )
        .expect("empty baseline");
    state
        .queue(policy(), upsert("a", "mutation-a", None))
        .expect("new entity");
    let rejected = MutationOutcome::Rejected {
        idempotency_key: replay("mutation-a"),
        code: FailureCode::new("lesson.invalid").expect("valid failure code"),
        retryable: false,
    };
    state
        .apply_push(policy(), SyncPushResult::new(None, vec![rejected]), 1_001)
        .expect("permanent rejection");
    assert_eq!(state.conflicts().len(), 1);

    state
        .resolve_conflict(
            policy(),
            &replay("mutation-a"),
            ConflictResolution::RetryAgainstServer {
                idempotency_key: replay("mutation-a-retry"),
            },
        )
        .expect("retry without a cached record");
    assert!(state.conflicts().is_empty());
    assert_eq!(state.pending()[0].base_revision(), None);
}
