use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

const KEY: &[u8] = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const PAYLOAD_BYTES: usize = 50;

fn gate(ttl_seconds: u64, capacity: usize) -> ProofOfWorkGate {
    ProofOfWorkGate::try_new(
        KEY,
        ProofOfWorkConfig::try_new(8, Duration::from_secs(ttl_seconds), capacity)
            .expect("PoW config"),
    )
    .expect("PoW gate")
}

fn solve_token(token: &str, difficulty_bits: u8) -> u64 {
    let bytes = decode_token(token).expect("token decodes");
    (0..1_000_000)
        .find(|nonce| proof_matches(&bytes, *nonce, difficulty_bits))
        .expect("bounded solution")
}

/// Re-signs `token` with the gate key after `edit` changes its payload, the
/// way a holder of the key (and only such a holder) could mint a token.
fn reissue(token: &str, edit: impl FnOnce(&mut [u8])) -> String {
    let mut bytes = URL_SAFE_NO_PAD.decode(token).expect("token encoding");
    bytes.truncate(PAYLOAD_BYTES);
    edit(&mut bytes);
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(KEY).expect("HMAC key");
    mac.update(b"rullst.sentinel.pow-token.v1");
    mac.update(&bytes);
    bytes.extend_from_slice(&mac.finalize().into_bytes());
    URL_SAFE_NO_PAD.encode(bytes)
}

fn shift_times(payload: &mut [u8], delta: u64) {
    for range in [33..41, 41..49] {
        let value = u64::from_be_bytes(payload[range.clone()].try_into().expect("epoch"));
        payload[range].copy_from_slice(&(value + delta).to_be_bytes());
    }
}

#[test]
fn config_and_challenge_accessors_report_the_policy() {
    let config = ProofOfWorkConfig::try_new(9, Duration::from_secs(45), 7).expect("config");
    assert_eq!(config.difficulty_bits(), 9);
    assert_eq!(config.ttl(), Duration::from_secs(45));
    assert_eq!(config.max_active_challenges(), 7);

    let gate = ProofOfWorkGate::try_new(KEY, config).expect("gate");
    let challenge = gate
        .issue_at("peer:a".to_string(), 1_000)
        .expect("challenge");
    assert_eq!(challenge.difficulty_bits(), 9);
    assert_eq!(challenge.expires_at_epoch(), 1_045);
}

#[test]
fn debug_output_shows_policy_and_load_but_not_the_key() {
    let gate = gate(30, 3);
    gate.issue_at("peer:a".to_string(), 100).expect("challenge");
    let rendered = format!("{gate:?}");
    assert!(rendered.starts_with("ProofOfWorkGate"));
    assert!(rendered.contains("active_challenges: 1"));
    assert!(rendered.contains("difficulty_bits: 8"));
    assert!(!rendered.contains("0123456789abcdef"));
}

#[test]
fn wall_clock_issue_and_verify_consume_the_challenge_once() {
    let gate = gate(30, 3);
    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let challenge = gate.issue("peer:a").expect("challenge");
    assert!(challenge.expires_at_epoch() >= before + 30);

    let nonce = solve_token(challenge.token(), 8);
    assert_eq!(
        gate.verify("peer:b", challenge.token(), nonce),
        Err(SentinelError::InvalidToken)
    );
    assert_eq!(gate.verify("peer:a", challenge.token(), nonce), Ok(()));
    assert_eq!(
        gate.verify("peer:a", challenge.token(), nonce),
        Err(SentinelError::ReplayOrUnknownChallenge)
    );
}

fn foreign_version(payload: &mut [u8]) {
    payload[0] = 2;
}

fn foreign_difficulty(payload: &mut [u8]) {
    payload[49] = 9;
}

fn longer_lifetime(payload: &mut [u8]) {
    let end = u64::from_be_bytes(payload[41..49].try_into().expect("epoch"));
    payload[41..49].copy_from_slice(&(end + 1).to_be_bytes());
}

type PayloadEdit = fn(&mut [u8]);

#[test]
fn reissued_tokens_with_a_foreign_header_are_rejected() {
    let gate = gate(30, 3);
    let challenge = gate.issue_at("peer:a".into(), 1_000).expect("challenge");
    let cases: [(PayloadEdit, u8); 3] = [
        (foreign_version, 8),
        (foreign_difficulty, 9),
        (longer_lifetime, 8),
    ];
    for (edit, difficulty) in cases {
        let token = reissue(challenge.token(), edit);
        let nonce = solve_token(&token, difficulty);
        assert_eq!(
            gate.verify_at("peer:a", &token, nonce, 1_001),
            Err(SentinelError::InvalidToken)
        );
    }
    assert_eq!(gate.active_challenges(), 1);
}

#[test]
fn challenges_are_valid_from_the_issue_second_until_expiry() {
    let gate = gate(30, 3);
    let early = gate.issue_at("peer:a".into(), 1_000).expect("challenge");
    let nonce = solve_token(early.token(), 8);
    assert_eq!(
        gate.verify_at("peer:a", early.token(), nonce, 999),
        Err(SentinelError::ExpiredChallenge)
    );

    let current = gate.issue_at("peer:a".into(), 1_000).expect("challenge");
    let nonce = solve_token(current.token(), 8);
    assert_eq!(
        gate.verify_at("peer:a", current.token(), nonce, 1_000),
        Ok(())
    );
}

#[test]
fn consumption_requires_the_exact_issued_subject_and_expiry() {
    let gate = gate(30, 3);
    let challenge = gate.issue_at("peer:a".into(), 1_000).expect("challenge");
    let foreign_tag = subject_tag(KEY, "peer:b").expect("subject tag");
    let foreign = reissue(challenge.token(), |payload| {
        payload[17..33].copy_from_slice(&foreign_tag);
    });
    let nonce = solve_token(&foreign, 8);
    assert_eq!(
        gate.verify_at("peer:b", &foreign, nonce, 1_001),
        Err(SentinelError::ReplayOrUnknownChallenge)
    );
    // An expired presentation for another subject must not evict the original.
    assert_eq!(
        gate.verify_at("peer:b", &foreign, nonce, 1_030),
        Err(SentinelError::ExpiredChallenge)
    );
    assert_eq!(gate.active_challenges(), 1);

    let later = reissue(challenge.token(), |payload| shift_times(payload, 1));
    let nonce = solve_token(&later, 8);
    assert_eq!(
        gate.verify_at("peer:a", &later, nonce, 1_031),
        Err(SentinelError::ExpiredChallenge)
    );
    assert_eq!(gate.active_challenges(), 1);
}

#[test]
fn cleanup_runs_once_thirty_seconds_have_passed() {
    let gate = gate(5, 1);
    gate.issue_at("peer:a".into(), 100)
        .expect("first challenge");
    assert_eq!(
        gate.issue_at("peer:b".into(), 110),
        Err(SentinelError::CapacityReached)
    );
    assert!(gate.issue_at("peer:b".into(), 130).is_ok());
}

#[test]
fn keys_need_thirty_two_bytes_and_twelve_distinct_values() {
    let twelve_distinct = b"0123456789ab".repeat(3);
    let eleven_distinct = b"0123456789a".repeat(3);
    let cases: [(&[u8], bool); 5] = [
        (&twelve_distinct[..32], true),
        (&twelve_distinct[..31], false),
        (b"0123456789abcdefghijklmnopqrstu", false),
        (&eleven_distinct[..32], false),
        (&[b'a'; 64], false),
    ];
    for (key, accepted) in cases {
        let result = ProofOfWorkGate::try_new(key, ProofOfWorkConfig::default());
        assert_eq!(result.is_ok(), accepted, "key length {}", key.len());
        if !accepted {
            assert_eq!(result.err(), Some(SentinelError::WeakKey));
        }
    }
}

#[test]
fn subjects_must_be_trimmed_bounded_and_printable() {
    let gate = gate(30, 10);
    for subject in [
        String::new(),
        " peer".into(),
        "a".repeat(257),
        "peer\u{7}".into(),
    ] {
        assert_eq!(
            gate.issue_at(subject, 100),
            Err(SentinelError::InvalidSubject)
        );
    }
    assert!(gate.issue_at("a".repeat(256), 100).is_ok());
}
