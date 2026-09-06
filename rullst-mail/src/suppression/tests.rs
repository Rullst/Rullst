use super::*;
use crate::drivers::MemoryDriver;

fn event(
    provider: &str,
    event_id: &str,
    recipient: &str,
    reason: SuppressionReason,
    observed_at: u64,
) -> SuppressionEvent {
    SuppressionEvent::try_new(provider, event_id, recipient, reason, observed_at)
        .expect("valid suppression event")
}

#[tokio::test]
async fn process_local_store_is_bounded_idempotent_and_reason_monotonic() {
    let now = unix_time().expect("clock");
    let store = InMemorySuppressionStore::new(1, 3).expect("bounded store");
    let manual = event(
        "admin",
        "manual-1",
        "Alice@Example.COM",
        SuppressionReason::Manual,
        now - 3,
    );
    let first = store.record(manual.clone()).await.expect("manual event");
    assert_eq!(first.recipient(), "Alice@example.com");
    assert_eq!(first.reason(), SuppressionReason::Manual);
    assert_eq!(store.record(manual).await.expect("exact replay"), first);
    let complaint = event(
        "postmark",
        "complaint-1",
        "Alice@example.com",
        SuppressionReason::SpamComplaint,
        now - 1,
    );
    let strongest = store.record(complaint).await.expect("complaint event");
    assert_eq!(strongest.reason(), SuppressionReason::SpamComplaint);
    assert_eq!(strongest.provider(), "postmark");

    let old_bounce = event(
        "ses",
        "bounce-1",
        "Alice@example.com",
        SuppressionReason::HardBounce,
        now - 2,
    );
    let after_old_bounce = store.record(old_bounce).await.expect("older bounce");
    assert_eq!(after_old_bounce.reason(), SuppressionReason::SpamComplaint);
    assert_eq!(after_old_bounce.provider(), "postmark");
    assert_eq!(store.snapshot().expect("snapshot").events(), 3);
    assert_eq!(store.snapshot().expect("snapshot").recipients(), 1);
    assert_eq!(
        store
            .record(event(
                "ses",
                "bounce-2",
                "bob@example.com",
                SuppressionReason::HardBounce,
                now,
            ))
            .await,
        Err(SuppressionError::CapacityExceeded)
    );
    assert_eq!(store.prune_events_before(now - 1).expect("prune"), 2);
    assert_eq!(
        store.prune_events_before(0),
        Err(SuppressionError::InvalidConfiguration("event cutoff"))
    );
    assert!(store.lookup("Alice@EXAMPLE.com").await.unwrap().is_some());
}

#[tokio::test]
async fn provider_event_conflicts_and_ambiguous_inputs_fail_closed() {
    let now = unix_time().expect("clock");
    let store = InMemorySuppressionStore::new(4, 4).expect("bounded store");
    store
        .record(event(
            "resend",
            "evt-7",
            "alice@example.com",
            SuppressionReason::HardBounce,
            now,
        ))
        .await
        .expect("first event");
    assert_eq!(
        store
            .record(event(
                "resend",
                "evt-7",
                "bob@example.com",
                SuppressionReason::HardBounce,
                now,
            ))
            .await,
        Err(SuppressionError::EventConflict)
    );
    for invalid in [
        " alice@example.com",
        "alice@example.com ",
        "álîçé@example.com",
    ] {
        assert!(
            SuppressionEvent::try_new(
                "provider",
                "event-1",
                invalid,
                SuppressionReason::Manual,
                now,
            )
            .is_err()
        );
    }
    assert!(
        SuppressionEvent::try_new(
            "bad/provider",
            "event-1",
            "alice@example.com",
            SuppressionReason::Manual,
            now,
        )
        .is_err()
    );
    let debug = format!(
        "{:?}",
        event(
            "provider",
            "secret-event-id",
            "alice@example.com",
            SuppressionReason::Manual,
            now,
        )
    );
    assert!(!debug.contains("secret-event-id"));
    assert!(!debug.contains("alice@example.com"));
}

struct UnavailableStore;

impl SuppressionStore for UnavailableStore {
    async fn lookup(
        &self,
        _recipient: &str,
    ) -> Result<Option<SuppressionRecord>, SuppressionError> {
        Err(SuppressionError::StorageUnavailable("offline fixture"))
    }
}

#[tokio::test]
async fn guard_blocks_suppressed_recipients_and_store_failure_before_transport() {
    let now = unix_time().expect("clock");
    let store = InMemorySuppressionStore::new(4, 4).expect("bounded store");
    store
        .record(event(
            "sendgrid",
            "complaint-9",
            "blocked@example.com",
            SuppressionReason::SpamComplaint,
            now,
        ))
        .await
        .expect("suppression");
    let (driver, deliveries) = MemoryDriver::isolated();
    let guard = SuppressionGuard::new(driver, store);
    let message = Message::new()
        .to("blocked@EXAMPLE.com")
        .subject("must not leave")
        .text("blocked");
    assert_eq!(
        guard.send(&message).await,
        Err(MailError::SuppressedRecipient {
            reason: "spam_complaint"
        })
    );
    assert!(deliveries.lock().expect("deliveries").is_empty());

    let (driver, deliveries) = MemoryDriver::isolated();
    let unavailable = SuppressionGuard::new(driver, UnavailableStore);
    assert_eq!(
        unavailable.send(&message).await,
        Err(MailError::SuppressionUnavailable)
    );
    assert!(deliveries.lock().expect("deliveries").is_empty());
}

#[test]
fn suppression_contract_exposes_stable_labels_bounds_and_minimized_errors() {
    assert_eq!(SuppressionReason::Manual.as_str(), "manual");
    assert_eq!(SuppressionReason::HardBounce.as_str(), "hard_bounce");
    assert_eq!(SuppressionReason::SpamComplaint.as_str(), "spam_complaint");

    #[cfg(feature = "sqlite")]
    {
        assert_eq!(
            SuppressionReason::from_rank(1),
            Ok(SuppressionReason::Manual)
        );
        assert_eq!(
            SuppressionReason::from_rank(2),
            Ok(SuppressionReason::HardBounce)
        );
        assert_eq!(
            SuppressionReason::from_rank(3),
            Ok(SuppressionReason::SpamComplaint)
        );
        assert_eq!(
            SuppressionReason::from_rank(4),
            Err(SuppressionError::CorruptStorage("suppression reason"))
        );
    }

    for (error, expected) in [
        (
            SuppressionError::InvalidConfiguration("fixture"),
            "invalid suppression configuration: fixture",
        ),
        (
            SuppressionError::InvalidEvent("fixture"),
            "invalid suppression event: fixture",
        ),
        (
            SuppressionError::EventConflict,
            "provider event identifier was reused with different contents",
        ),
        (
            SuppressionError::CapacityExceeded,
            "suppression store capacity is exhausted",
        ),
        (
            SuppressionError::StorageUnavailable("fixture"),
            "suppression storage operation failed: fixture",
        ),
        (
            SuppressionError::CorruptStorage("fixture"),
            "suppression storage is corrupt: fixture",
        ),
    ] {
        assert_eq!(error.to_string(), expected);
    }
    assert_eq!(
        unavailable("lookup"),
        SuppressionError::StorageUnavailable("lookup")
    );

    for (recipients, events) in [(0, 1), (1, 0), (MAX_STORE_ENTRIES + 1, 1)] {
        assert_eq!(
            validate_limits(recipients, events),
            Err(SuppressionError::InvalidConfiguration("store limits"))
        );
    }
}

#[tokio::test]
async fn event_record_and_guard_accessors_preserve_the_bounded_contract() {
    let now = unix_time().expect("clock");
    for observed_at in [0, now.saturating_add(MAX_FUTURE_SKEW_SECONDS + 1)] {
        assert_eq!(
            SuppressionEvent::try_new(
                "provider",
                "event",
                "reader@example.com",
                SuppressionReason::Manual,
                observed_at,
            ),
            Err(SuppressionError::InvalidEvent("observation time"))
        );
    }

    let store = InMemorySuppressionStore::new(2, 2).expect("bounded store");
    let suppression = event(
        "operator",
        "manual-reader",
        "Reader@Example.COM",
        SuppressionReason::Manual,
        now,
    );
    assert_eq!(suppression.provider(), "operator");
    assert_eq!(suppression.event_id(), "manual-reader");
    assert_eq!(suppression.recipient(), "Reader@example.com");
    assert_eq!(suppression.reason(), SuppressionReason::Manual);
    assert_eq!(suppression.observed_at(), now);

    let record = store.record(suppression).await.expect("record");
    assert_eq!(record.first_seen_at(), now);
    assert_eq!(record.last_seen_at(), now);
    let record_debug = format!("{record:?}");
    assert!(!record_debug.contains("Reader@example.com"));
    assert!(record_debug.contains("operator"));

    let (driver, deliveries) = MemoryDriver::isolated();
    let guard = SuppressionGuard::new(driver, InMemorySuppressionStore::new(2, 2).expect("store"));
    assert_eq!(
        guard.store().snapshot().expect("snapshot").max_recipients(),
        2
    );
    assert_eq!(guard.store().snapshot().expect("snapshot").max_events(), 2);
    let _ = guard.driver();
    guard
        .send_for_tenant(
            "tenant_acme",
            &Message::new()
                .to("allowed@example.com")
                .subject("allowed")
                .text("bounded"),
        )
        .await
        .expect("tenant delivery");
    assert_eq!(deliveries.lock().expect("deliveries").len(), 1);
}
