use super::super::SiemIntegrityKey;
use super::*;

fn material() -> Vec<u8> {
    (0_u8..32).collect()
}

fn keys(id: &str) -> SiemKeyRing {
    let key = SiemIntegrityKey::try_new(id, material()).expect("valid test key");
    SiemKeyRing::try_new(key, []).expect("valid test ring")
}

fn sample_event() -> LiveSecurityEvent {
    LiveSecurityEvent::local("SECURITY_EVENT", "framed", "192.0.2.20")
}

fn payload(event: &LiveSecurityEvent) -> Vec<u8> {
    serde_json::to_vec(event).expect("encode event")
}

/// One signed frame without its trailing newline, as `decode_frame` sees it.
fn signed_frame(sequence: u64, key_id: &str, payload: &[u8]) -> Vec<u8> {
    let (mut frame, _) =
        encode_frame(sequence, key_id, &material(), [0; 32], payload).expect("signed frame");
    frame.pop();
    frame
}

fn decode_first(
    frame: &[u8],
    keys: &SiemKeyRing,
) -> Result<LiveSecurityEvent, AuthenticatedSiemSpoolError> {
    decode_frame(frame, 1, 1, [0; 32], keys).map(|(event, _)| event)
}

fn reason(result: Result<LiveSecurityEvent, AuthenticatedSiemSpoolError>) -> &'static str {
    match result {
        Err(AuthenticatedSiemSpoolError::CorruptRecord { record: 1, reason }) => reason,
        other => panic!("expected a corrupt first record, got {other:?}"),
    }
}

#[test]
fn records_hold_payloads_up_to_sixteen_kib() {
    let limit = 16 * 1024;
    assert!(encode_frame(1, "k", &material(), [0; 32], &vec![b'x'; limit]).is_ok());
    assert_eq!(
        encode_frame(1, "k", &material(), [0; 32], &vec![b'x'; limit + 1]).err(),
        Some(AuthenticatedSiemSpoolError::RecordTooLarge)
    );

    let ring = keys("k");
    let frame = signed_frame(1, "k", &payload(&sample_event()));
    let mut at_limit = frame.clone();
    at_limit[..8].copy_from_slice(b"00004000");
    assert_eq!(
        reason(decode_first(&at_limit, &ring)),
        "record length mismatch"
    );
    let mut over_limit = frame;
    over_limit[..8].copy_from_slice(b"00004001");
    assert_eq!(
        reason(decode_first(&over_limit, &ring)),
        "record length exceeds limit"
    );
}

#[test]
fn frames_shorter_than_the_fixed_header_are_rejected_first() {
    let ring = keys("k");
    let frame = signed_frame(1, "k", &payload(&sample_event()));
    assert!(frame.len() > 290);
    assert_eq!(
        reason(decode_first(&frame[..159], &ring)),
        "invalid frame header"
    );
    assert_eq!(
        reason(decode_first(&frame[..160], &ring)),
        "invalid frame delimiter"
    );
    for length in [173, 263, 285] {
        assert_eq!(
            reason(decode_first(&frame[..length], &ring)),
            "record length mismatch",
            "{length} bytes"
        );
    }
}

#[test]
fn a_correctly_signed_frame_must_carry_the_expected_sequence() {
    let ring = keys("k");
    let frame = signed_frame(2, "k", &payload(&sample_event()));
    assert_eq!(
        reason(decode_first(&frame, &ring)),
        "non-canonical or discontinuous sequence"
    );
}

#[test]
fn key_identifier_lengths_are_one_to_thirty_two_canonical_hex() {
    let longest = "a".repeat(32);
    let event = sample_event();
    let mut decoded = decode_first(
        &signed_frame(1, &longest, &payload(&event)),
        &keys(&longest),
    )
    .expect("32-byte key identifier");
    decoded.verified_hmac = false;
    assert_eq!(decoded, event);

    let ten = "key-abcdef";
    let mut uppercase = signed_frame(1, ten, &payload(&event));
    assert_eq!(&uppercase[26..28], b"0a");
    uppercase[26..28].copy_from_slice(b"0A");
    assert_eq!(
        reason(decode_first(&uppercase, &keys(ten))),
        "invalid key identifier length"
    );

    let body = payload(&event);
    let mut empty_key = format!(
        "{:08x}:{:016x}:00::{}:{}:",
        body.len(),
        1,
        "0".repeat(64),
        "0".repeat(64)
    )
    .into_bytes();
    empty_key.extend_from_slice(&body);
    assert_eq!(
        reason(decode_first(&empty_key, &keys("k"))),
        "invalid key identifier length"
    );
}

#[test]
fn the_length_field_must_match_the_payload_exactly() {
    let ring = keys("k");
    let body = payload(&sample_event());
    let mut frame = signed_frame(1, "k", &body);
    frame[..8].copy_from_slice(format!("{:08x}", body.len() - 1).as_bytes());
    assert_eq!(
        reason(decode_first(&frame, &ring)),
        "record length mismatch"
    );
}

#[test]
fn signed_events_must_already_be_normalized_and_unsigned() {
    let mut event = sample_event();
    event.verified_hmac = true;
    let frame = signed_frame(1, "k", &payload(&event));
    assert_eq!(
        reason(decode_first(&frame, &keys("k"))),
        "event violates the unsigned local v1 contract"
    );
}
