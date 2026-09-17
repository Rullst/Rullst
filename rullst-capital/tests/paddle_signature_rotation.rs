use ring::hmac;
use rullst_capital::providers::paddle::PaddleProvider;

const SECRET: &str = "pdl_ntfset_fixture_secret";
const NOW: i64 = 1_800_000_000;
const BODY: &[u8] = br#"{"event_type":"subscription.updated"}"#;

fn signature() -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, SECRET.as_bytes());
    let mut context = hmac::Context::with_key(&key);
    context.update(format!("{NOW}:").as_bytes());
    context.update(BODY);
    hex::encode(context.sign().as_ref())
}

#[test]
fn accepts_a_valid_h1_in_any_position_without_skipping_raw_body_or_freshness_checks() {
    let provider = PaddleProvider::new("unused_fixture_key", SECRET);
    let valid = signature();
    let wrong = "00".repeat(32);
    for candidates in [
        format!("h1={valid};h1={wrong};h1=malformed"),
        format!("h1={wrong};h1={valid};h1=malformed"),
        format!("h1=malformed;h1={wrong};h1={valid}"),
    ] {
        let header = format!("ts={NOW};{candidates}");
        assert!(provider.verify_signature_at(BODY, &header, NOW).is_ok());
        assert!(
            provider
                .verify_signature_at(b"changed", &header, NOW)
                .is_err()
        );
        assert!(
            provider
                .verify_signature_at(BODY, &header, NOW + 301)
                .is_err()
        );
        assert!(
            provider
                .verify_signature_at(BODY, &header, NOW - 301)
                .is_err()
        );
    }
}

#[test]
fn rejects_ambiguous_timestamps_and_bounded_candidate_overflow_even_with_a_valid_signature() {
    let provider = PaddleProvider::new("unused_fixture_key", SECRET);
    let valid = signature();
    for header in [
        format!("ts={NOW};ts={NOW};h1={valid}"),
        format!("ts=invalid;ts={NOW};h1={valid}"),
        format!("ts={NOW};h1={valid};{}", "h1=bad;".repeat(16)),
        format!("ts={NOW};h1={valid};v0={}", "x".repeat(4096)),
        format!("ts={NOW};h1={};h1=malformed", "00".repeat(32)),
    ] {
        assert!(provider.verify_signature_at(BODY, &header, NOW).is_err());
    }
}
