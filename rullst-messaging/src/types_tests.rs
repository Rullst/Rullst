#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

#[test]
fn stored_message_ids_need_both_the_length_and_the_prefix() {
    let valid = "msg_0123456789abcdef0123456789abcdef";
    assert_eq!(
        MessageId::from_stored(valid.to_owned()).unwrap().as_str(),
        valid
    );
    for invalid in ["msg_abc", "xyz_0123456789abcdef0123456789abcdef"] {
        assert!(
            MessageId::from_stored(invalid.to_owned()).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn ack_tokens_never_print_their_value() {
    assert_eq!(format!("{:?}", AckToken::random()), "AckToken([REDACTED])");
}

#[test]
fn headers_may_use_exactly_eight_kib_of_names_and_values() {
    let mut headers = MessageHeaders::new();
    assert!(headers.is_empty());
    for index in 0..8 {
        // 2 + 1_022 bytes per header.
        headers
            .try_insert(format!("h{index}"), "v".repeat(1_022))
            .unwrap();
    }
    assert!(!headers.is_empty());
    assert_eq!(headers.len(), 8);
    assert_eq!(
        headers.try_insert("x", ""),
        Err(MessagingError::CapacityExceeded {
            resource: "message header bytes",
            limit: 8_192,
        })
    );
}

#[test]
fn payload_limits_default_to_one_mib_and_stop_at_sixteen() {
    let config = BrokerConfig::try_new("limits").unwrap();
    assert_eq!(config.max_payload_bytes(), 1_048_576);
    assert!(config.clone().with_limits(1, 1, 1, 16_777_216).is_ok());
    assert!(config.with_limits(1, 1, 1, 16_777_217).is_err());
}
