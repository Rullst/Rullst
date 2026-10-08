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
