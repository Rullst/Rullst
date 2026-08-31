use super::*;

#[test]
fn conflict_retry_requires_current_authoritative_state_after_recovery() {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .replace_server_snapshot(
            policy(),
            vec![record("a", 1, 10)],
            SyncCursor::new("cursor-1").expect("cursor"),
            1_000,
        )
        .expect("baseline");
    state
        .queue(policy(), upsert("a", "mutation-a", Some(1)))
        .expect("local mutation");
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
        .expect("server update");
    assert_eq!(state.conflicts()[0].authoritative_revision(), Some(2));

    state.recover_server_cache();
    let before = state.clone();
    assert!(matches!(
        state.resolve_conflict(
            policy(),
            &replay("mutation-a"),
            ConflictResolution::RetryAgainstServer {
                idempotency_key: replay("mutation-b")
            }
        ),
        Err(OfflineSyncError::FullResyncRequired)
    ));
    assert_eq!(state, before);

    state
        .replace_server_snapshot(
            policy(),
            vec![record("a", 3, 30)],
            SyncCursor::new("cursor-3").expect("cursor"),
            1_002,
        )
        .expect("fresh authority");
    assert_eq!(state.conflicts()[0].authoritative_revision(), Some(3));
    state
        .resolve_conflict(
            policy(),
            &replay("mutation-a"),
            ConflictResolution::RetryAgainstServer {
                idempotency_key: replay("mutation-b"),
            },
        )
        .expect("retry against current revision");
    assert_eq!(state.pending()[0].base_revision(), Some(3));
}
