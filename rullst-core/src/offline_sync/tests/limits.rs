//! Exact boundaries of the policy ceilings, bounded JSON and the encrypted
//! envelope.

use super::*;

fn limits(max_payload_bytes: usize, max_snapshot_bytes: usize) -> OfflineSyncPolicy {
    OfflineSyncPolicy::new(8, 6, 6, max_payload_bytes, max_snapshot_bytes, 3).expect("policy")
}

fn pending_state() -> OfflineSyncState {
    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(policy(), upsert("a", "mutation-a", None))
        .expect("pending mutation");
    state
}

/// An envelope of `len` bytes that names the one-byte key id `b`.
fn envelope_for_key_b(len: usize) -> Vec<u8> {
    let mut envelope = b"RLSOFF01\x01b".to_vec();
    envelope.resize(len, 0);
    envelope
}

#[test]
fn policy_ceilings_and_accessors_are_exact() {
    assert!(OfflineSyncPolicy::new(1, 1, 1, 1, 33_554_432, 1).is_ok());
    assert!(matches!(
        OfflineSyncPolicy::new(1, 1, 1, 1, 33_554_433, 1),
        Err(OfflineSyncError::InvalidPolicy)
    ));
    let policy = OfflineSyncPolicy::new(8, 6, 7, 2_048, 4_096, 3).expect("policy");
    assert_eq!(policy.max_conflicts(), 7);
    assert_eq!(policy.max_snapshot_bytes(), 4_096);
}

#[test]
fn cipher_key_ids_are_bounded_tokens() {
    let key = [7_u8; 32];
    for invalid in [String::new(), "k".repeat(65), "bad id".to_owned()] {
        assert!(matches!(
            OfflineSnapshotCipher::new(invalid, key),
            Err(OfflineSyncError::InvalidKeyId)
        ));
    }
    for valid in ["k".repeat(64), "rotation-2026_10.a".to_owned()] {
        let cipher = OfflineSnapshotCipher::new(valid.clone(), key).expect("valid key id");
        assert_eq!(cipher.key_id(), valid);
    }
}

#[test]
fn envelopes_may_reach_the_snapshot_limit_plus_their_fixed_overhead() {
    // 4,096 plaintext bytes, 37 fixed envelope bytes and a 64-byte key id.
    let policy = limits(2_048, 4_096);
    let cipher = OfflineSnapshotCipher::new("a", [7_u8; 32]).expect("cipher");
    let open = |len: usize| cipher.open(policy, &account("user-1"), &envelope_for_key_b(len));
    assert!(matches!(
        open(4_198),
        Err(OfflineSyncError::SnapshotTooLarge { maximum: 4_197 })
    ));
    for len in [4_197, 37] {
        assert!(
            matches!(open(len), Err(OfflineSyncError::SnapshotKeyIdMismatch)),
            "{len}"
        );
    }
    assert!(matches!(open(36), Err(OfflineSyncError::InvalidSnapshot)));
}

#[test]
fn a_snapshot_that_exactly_fills_every_bound_round_trips() {
    let state = pending_state();
    let plaintext = serde_json::to_vec(&state).expect("state JSON").len();
    let policy = limits(2_048, plaintext);
    let cipher = OfflineSnapshotCipher::new("k".repeat(64), [7_u8; 32]).expect("cipher");

    let envelope = cipher.seal(policy, &state).expect("snapshot at the limit");
    // Magic, length byte, key id, nonce and tag around the plaintext.
    assert_eq!(envelope.len(), plaintext + 101);
    assert_eq!(
        cipher
            .open(policy, &account("user-1"), &envelope)
            .expect("round trip"),
        state
    );

    let smaller = limits(2_048, plaintext - 1);
    assert!(matches!(
        cipher.seal(smaller, &state),
        Err(OfflineSyncError::SnapshotTooLarge { maximum }) if maximum == plaintext - 1
    ));
}

#[test]
fn a_payload_may_fill_its_limit_exactly() {
    let payload = json!({ "note": "x".repeat(100) });
    let encoded = serde_json::to_vec(&payload).expect("payload JSON").len();
    let mutation = || {
        OfflineMutation::upsert(replay("mutation-a"), entity("a"), None, 0, payload.clone())
            .expect("mutation")
    };

    let mut state = OfflineSyncState::new(account("user-1"));
    state
        .queue(limits(encoded, 32_768), mutation())
        .expect("payload at the limit");
    let mut state = OfflineSyncState::new(account("user-1"));
    assert!(matches!(
        state.queue(limits(encoded - 1, 32_768), mutation()),
        Err(OfflineSyncError::PayloadTooLarge { maximum }) if maximum == encoded - 1
    ));
}
