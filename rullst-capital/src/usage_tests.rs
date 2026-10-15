use super::*;

#[test]
fn stripe_event_validates_identity_quantity_time_and_redacts_debug() {
    let event = StripeMeterEvent::new_at(
        "cus_123",
        "lesson_minutes",
        7,
        1_000,
        "usage-event-123",
        1_000,
    )
    .expect("valid Stripe event");
    assert_eq!(event.customer_id(), "cus_123");
    assert_eq!(event.event_name(), "lesson_minutes");
    assert_eq!(event.value(), 7);
    assert_eq!(event.occurred_at(), 1_000);
    assert_eq!(event.identifier(), "usage-event-123");
    let debug = format!("{event:?}");
    assert!(!debug.contains("cus_123"));
    assert!(!debug.contains("usage-event-123"));

    for invalid in [
        StripeMeterEvent::new_at("customer", "metric", 1, 1_000, "key", 1_000),
        StripeMeterEvent::new_at("cus_1", "bad metric", 1, 1_000, "key", 1_000),
        StripeMeterEvent::new_at("cus_1", "metric", 0, 1_000, "key", 1_000),
        StripeMeterEvent::new_at(
            "cus_1",
            "metric",
            MAX_USAGE_QUANTITY + 1,
            1_000,
            "key",
            1_000,
        ),
        StripeMeterEvent::new_at("cus_1", "metric", 1, 1_000, "bad key", 1_000),
        StripeMeterEvent::new_at("cus_1", "metric", 1, 1_301, "key", 1_000),
        StripeMeterEvent::new_at(
            "cus_1",
            "metric",
            1,
            1_000 - STRIPE_MAX_PAST_SECONDS - 1,
            "key",
            1_000,
        ),
    ] {
        assert!(matches!(invalid, Err(CapitalError::InvalidUsage(_))));
    }
}

#[test]
fn receipt_and_mock_keep_status_deduplication_and_secrets_explicit() {
    let first = mock_usage_receipt("stripe", "event-key", 3, &["cus_1", "metric"])
        .expect("mock usage receipt");
    let second = mock_usage_receipt("stripe", "event-key", 3, &["cus_1", "metric"])
        .expect("mock usage receipt");
    assert_eq!(first, second);
    assert_eq!(first.provider(), "stripe");
    assert_eq!(first.event_key(), "event-key");
    assert_eq!(first.quantity(), 3);
    assert_eq!(first.status(), UsageStatus::Mock);
    assert_eq!(first.deduplication(), UsageDeduplication::Mock);
    assert!(!first.is_live_accepted());
    let debug = format!("{first:?}");
    assert!(!debug.contains("event-key"));
    assert!(!debug.contains(first.record_id()));

    assert!(matches!(
        UsageReceipt::from_verified_provider_response(
            "stripe",
            "record",
            "bad key",
            1,
            UsageStatus::Accepted,
            UsageDeduplication::ProviderRollingWindow,
        ),
        Err(CapitalError::InvalidUsage(_))
    ));
    assert!(matches!(
        UsageReceipt::from_verified_provider_response(
            "stripe",
            "record",
            "event-key",
            1,
            UsageStatus::Mock,
            UsageDeduplication::ProviderRollingWindow,
        ),
        Err(CapitalError::ProviderRequestFailed(_))
    ));

    // Custom adapters without a provider deduplication key keep the outbox contract.
    let outbox = UsageReceipt::from_verified_provider_response(
        "custom-gateway",
        "usage_1",
        "tenant-7:usage-99",
        5,
        UsageStatus::Accepted,
        UsageDeduplication::ApplicationOutboxRequired,
    )
    .expect("custom outbox receipt");
    assert!(outbox.is_live_accepted());
    assert_eq!(
        outbox.deduplication(),
        UsageDeduplication::ApplicationOutboxRequired
    );
}

#[test]
fn stripe_timestamp_window_is_35_days_back_and_5_minutes_ahead() {
    let now = 10_000_000;
    let event =
        |occurred_at| StripeMeterEvent::new_at("cus_1", "metric", 1, occurred_at, "key", now);
    event(now - 3_024_000).expect("exactly 35 days old");
    event(now + 300).expect("exactly 5 minutes ahead");
    assert!(matches!(
        event(now - 3_024_001),
        Err(CapitalError::InvalidUsage(_))
    ));
    assert!(matches!(
        event(now + 301),
        Err(CapitalError::InvalidUsage(_))
    ));
}

#[test]
fn quantity_reaches_the_largest_signed_64_bit_value() {
    let event = StripeMeterEvent::new_at("cus_1", "metric", 9_223_372_036_854_775_807, 1, "key", 1)
        .expect("largest quantity");
    assert_eq!(event.value(), 9_223_372_036_854_775_807);
}

fn receipt_with_record(record_id: &str) -> Result<UsageReceipt, CapitalError> {
    UsageReceipt::from_verified_provider_response(
        "stripe",
        record_id,
        "event-key",
        1,
        UsageStatus::Accepted,
        UsageDeduplication::ProviderRollingWindow,
    )
}

#[test]
fn provider_references_are_bounded_trimmed_and_printable() {
    let longest = "r".repeat(255);
    assert_eq!(receipt_with_record(&longest).unwrap().record_id(), longest);
    for invalid in [
        String::new(),
        "r".repeat(256),
        " record".to_string(),
        "rec\u{1}ord".to_string(),
    ] {
        assert!(
            matches!(
                receipt_with_record(&invalid),
                Err(CapitalError::InvalidUsage(_))
            ),
            "{invalid:?}"
        );
    }
    assert!(matches!(
        StripeMeterEvent::new_at("cus_1 ", "metric", 1, 1, "key", 1),
        Err(CapitalError::InvalidUsage(_))
    ));
}

#[test]
fn debug_output_keeps_public_fields() {
    let event = StripeMeterEvent::new_at("cus_123", "lesson_minutes", 7, 1_000, "key", 1_000)
        .expect("valid Stripe event");
    let debug = format!("{event:?}");
    assert!(debug.starts_with("StripeMeterEvent"), "{debug}");
    assert!(debug.contains("event_name: \"lesson_minutes\""), "{debug}");
    assert!(debug.contains("value: 7"), "{debug}");
    assert!(debug.contains("occurred_at: 1000"), "{debug}");
    let receipt = receipt_with_record("record_1").unwrap();
    let debug = format!("{receipt:?}");
    assert!(debug.starts_with("UsageReceipt"), "{debug}");
    assert!(debug.contains("provider: \"stripe\""), "{debug}");
    assert!(debug.contains("status: Accepted"), "{debug}");
    assert!(
        debug.contains("deduplication: ProviderRollingWindow"),
        "{debug}"
    );
    assert!(!debug.contains("record_1"), "{debug}");
}
