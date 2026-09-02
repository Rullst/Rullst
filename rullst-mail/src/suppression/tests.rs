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
