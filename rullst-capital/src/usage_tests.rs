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
fn lemon_record_validates_provider_relationship_and_application_key() {
    let record = LemonSqueezyUsageRecord::new(
        "42",
        "ai_exercises",
        5,
        LemonSqueezyUsageAction::Increment,
        "school-7:usage-99",
    )
    .expect("valid Lemon Squeezy usage record");
    assert_eq!(record.subscription_item_id(), "42");
    assert_eq!(record.application_metric(), "ai_exercises");
    assert_eq!(record.quantity(), 5);
    assert_eq!(record.action().as_str(), "increment");
    assert_eq!(record.event_key(), "school-7:usage-99");
    let debug = format!("{record:?}");
    assert!(!debug.contains("school-7:usage-99"));

    for invalid in [
        LemonSqueezyUsageRecord::new(
            "sub_42",
            "metric",
            1,
            LemonSqueezyUsageAction::Increment,
            "key",
        ),
        LemonSqueezyUsageRecord::new(
            "42",
            "bad metric",
            1,
            LemonSqueezyUsageAction::Increment,
            "key",
        ),
        LemonSqueezyUsageRecord::new("42", "metric", 0, LemonSqueezyUsageAction::Increment, "key"),
        LemonSqueezyUsageRecord::new("42", "metric", 1, LemonSqueezyUsageAction::Set, "bad key"),
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
}
