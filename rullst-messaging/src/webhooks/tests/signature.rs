use super::*;

const ID: &str = "msg_0123456789abcdef0123456789abcdef";
const SKEW: Duration = Duration::from_secs(300);

#[test]
fn offline_credentials_are_reported_and_never_verify() {
    let clock = ManualClock::at(START);
    for encoded in ["", "mock_receiver"] {
        let key = WebhookSigningKey::new("fixture", encoded).unwrap();
        assert!(key.is_offline(), "{encoded}");
        // Even a tag made with the offline placeholder secret is refused.
        let signature = key.sign(ID, "ready", START / 1000, b"{}").unwrap();
        assert_eq!(
            key.verify(&signature, b"{}", &clock, SKEW).err(),
            Some(WebhookError::InvalidSignature)
        );
    }
    assert!(!live_key().is_offline());
}

#[test]
fn verification_accepts_the_largest_body_and_the_full_skew_exactly() {
    let key = live_key();
    let clock = ManualClock::at(START);
    let body = vec![b' '; 65_536];
    let signature = key.sign(ID, "ready", START / 1000, &body).unwrap();
    let verified = key.verify(&signature, &body, &clock, SKEW).unwrap();
    assert_eq!(verified.delivery_id(), ID);
    assert_eq!(verified.event_kind(), "ready");
    assert_eq!(verified.timestamp_seconds(), 1_800_000_000);

    let longer = vec![b' '; 65_537];
    let signature = key.sign(ID, "ready", START / 1000, &longer).unwrap();
    assert_eq!(
        key.verify(&signature, &longer, &clock, SKEW).err(),
        Some(WebhookError::InvalidSignature)
    );

    for seconds in [1_799_999_700, 1_800_000_300] {
        let signature = key.sign(ID, "ready", seconds, b"{}").unwrap();
        assert_eq!(
            key.verify(&signature, b"{}", &clock, SKEW)
                .unwrap()
                .timestamp_seconds(),
            seconds
        );
    }
    for seconds in [1_799_999_699, 1_800_000_301] {
        let signature = key.sign(ID, "ready", seconds, b"{}").unwrap();
        assert_eq!(
            key.verify(&signature, b"{}", &clock, SKEW).err(),
            Some(WebhookError::InvalidSignature),
            "{seconds}"
        );
    }
}

#[test]
fn headers_accept_exact_bounds_and_reject_unsafe_key_ids() {
    let tag = live_key()
        .sign(ID, "ready", START / 1000, b"{}")
        .unwrap()
        .signature_header();
    let parse = |kind: &str, timestamp: &str, key_id: &str| {
        WebhookSignature::from_headers(ID, kind, timestamp, key_id, tag.clone())
    };

    let kind = "k".repeat(128);
    let parsed = parse(&kind, "1800000000", "fixture").unwrap();
    assert_eq!(parsed.event_kind(), kind);
    assert_eq!(parsed.delivery_id(), ID);
    assert_eq!(parsed.key_id(), "fixture");
    assert_eq!(parsed.signature_header(), tag);
    assert!(parse(&"k".repeat(129), "1800000000", "fixture").is_err());

    // The last second of year 9999 has twelve digits; the next one is out of range.
    let last = parse("ready", "253402300799", "fixture").unwrap();
    assert_eq!(last.timestamp_seconds(), 253_402_300_799);
    assert!(parse("ready", "253402300800", "fixture").is_err());

    for key_id in ["", "bad/id", "key id", &"k".repeat(65)] {
        assert_eq!(
            parse("ready", "1800000000", key_id).err(),
            Some(WebhookError::InvalidSignature),
            "{key_id:?}"
        );
    }
    assert_eq!(format!("{parsed:?}"), "WebhookSignature([REDACTED])");
}
