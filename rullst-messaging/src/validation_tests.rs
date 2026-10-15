#![allow(clippy::expect_used, clippy::unwrap_used)]

use crate::{ContentType, FailureCode, IdempotencyKey, MessageHeaders, Namespace};

#[test]
fn every_grammar_accepts_exactly_its_maximum_length() {
    assert!(Namespace::try_new("n".repeat(128)).is_ok());
    assert!(Namespace::try_new("n".repeat(129)).is_err());
    assert!(IdempotencyKey::try_new("k".repeat(255)).is_ok());
    assert!(IdempotencyKey::try_new("k".repeat(256)).is_err());
    assert!(ContentType::try_new(format!("a/{}", "b".repeat(125))).is_ok());
    assert!(ContentType::try_new(format!("a/{}", "b".repeat(126))).is_err());
    assert!(FailureCode::try_new("c".repeat(128)).is_ok());
    assert!(FailureCode::try_new("c".repeat(129)).is_err());
    let mut headers = MessageHeaders::new();
    assert!(headers.try_insert("h".repeat(64), "value").is_ok());
    assert!(headers.try_insert("h".repeat(65), "value").is_err());
}
