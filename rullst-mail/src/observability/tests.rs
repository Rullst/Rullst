use super::*;
use crate::drivers::MemoryDriver;

struct FailingDriver(MailError);

#[async_trait]
impl MailDriver for FailingDriver {
    async fn send(&self, _message: &Message) -> Result<(), MailError> {
        Err(self.0.clone())
    }
}

fn message(recipient: &str) -> Message {
    Message::new()
        .to(recipient)
        .subject("private subject")
        .text("private body")
}

#[tokio::test]
// TM-MAIL-03: terminal observations exclude recipient and message content.
async fn observations_classify_terminal_outcomes_without_message_content() {
    let observer = BoundedMailObserver::new(4).expect("observer");
    let (driver, _) = MemoryDriver::isolated();
    let successful =
        ObservedMailDriver::try_new("memory", driver, observer.clone()).expect("wrapper");
    successful
        .send(&message("alice@example.com"))
        .await
        .expect("delivery");

    let transient = ObservedMailDriver::try_new(
        "fixture",
        FailingDriver(MailError::transport("fixture", "offline")),
        observer.clone(),
    )
    .expect("transient wrapper");
    assert!(transient.send(&message("bob@example.com")).await.is_err());

    let invalid = message("not-an-email");
    assert!(successful.send(&invalid).await.is_err());
    let snapshot = observer.snapshot().expect("snapshot");
    assert_eq!(snapshot.observations().len(), 3);
    assert_eq!(
        snapshot.observations()[0].outcome(),
        MailDeliveryOutcome::Delivered
    );
    assert_eq!(
        snapshot.observations()[1].outcome(),
        MailDeliveryOutcome::TransientFailure
    );
    assert_eq!(
        snapshot.observations()[2].outcome(),
        MailDeliveryOutcome::PreflightRejected
    );
    let debug = format!("{snapshot:?}");
    for private in [
        "alice@example.com",
        "bob@example.com",
        "private subject",
        "private body",
    ] {
        assert!(!debug.contains(private));
    }
}

#[tokio::test]
async fn local_observer_is_bounded_and_marks_tenant_dispatch() {
    let observer = BoundedMailObserver::new(1).expect("observer");
    let (driver, deliveries) = MemoryDriver::isolated();
    let observed =
        ObservedMailDriver::try_new("memory", driver, observer.clone()).expect("wrapper");
    observed
        .send(&message("first@example.com"))
        .await
        .expect("first");
    observed
        .send_for_tenant("tenant-a", &message("second@example.com"))
        .await
        .expect("tenant delivery");
    let snapshot = observer.snapshot().expect("snapshot");
    assert_eq!(snapshot.capacity(), 1);
    assert_eq!(snapshot.evicted(), 1);
    assert_eq!(snapshot.observations().len(), 1);
    assert!(snapshot.observations()[0].tenant_scoped());
    assert_eq!(snapshot.observations()[0].provider(), "memory");
    assert_eq!(deliveries.lock().expect("deliveries").len(), 2);
}

#[test]
fn observer_configuration_is_bounded_and_provider_labels_are_low_cardinality() {
    assert!(matches!(
        BoundedMailObserver::new(0),
        Err(MailObservationError::InvalidCapacity)
    ));
    let (driver, _) = MemoryDriver::isolated();
    let observer = BoundedMailObserver::new(4).expect("observer");
    assert!(ObservedMailDriver::try_new("bad/provider", driver, observer).is_err());
}

struct SlowDriver;

#[async_trait]
impl MailDriver for SlowDriver {
    async fn send(&self, _message: &Message) -> Result<(), MailError> {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        Ok(())
    }
}

#[tokio::test]
async fn observations_report_elapsed_time_attachments_schedule_and_tenant() {
    let observer = BoundedMailObserver::new(4).expect("observer");
    let observed = ObservedMailDriver::try_new("slow", SlowDriver, observer.clone()).expect("slow");
    let attached = message("first@example.com")
        .attach_bytes("a.txt", b"a".to_vec(), "text/plain")
        .attach_bytes("b.txt", b"b".to_vec(), "text/plain");
    observed.send(&attached).await.expect("attached delivery");
    observed
        .send(&message("second@example.com").send_in(std::time::Duration::from_secs(60)))
        .await
        .expect("scheduled delivery");

    let snapshot = observer.snapshot().expect("snapshot");
    let [first, second] = snapshot.observations() else {
        panic!("two observations expected");
    };
    // The driver sleeps 5 ms, so at least 5,000 microseconds elapse.
    assert!(first.elapsed_microseconds() >= 5_000);
    assert_eq!(first.attachment_count(), 2);
    assert!(!first.scheduled());
    assert!(!first.tenant_scoped());
    assert_eq!(second.attachment_count(), 0);
    assert!(second.scheduled());
    assert_eq!(snapshot.capacity(), 4);
    assert_eq!(snapshot.evicted(), 0);
}

#[tokio::test]
async fn eviction_counts_every_record_dropped_at_capacity() {
    let observer = BoundedMailObserver::new(1).expect("observer");
    let (driver, _) = MemoryDriver::isolated();
    let observed = ObservedMailDriver::try_new("memory", driver, observer.clone()).expect("memory");
    for recipient in ["a@example.com", "b@example.com", "c@example.com"] {
        observed.send(&message(recipient)).await.expect("delivery");
    }
    assert_eq!(observer.snapshot().expect("snapshot").evicted(), 2);
}

#[test]
fn provider_labels_hold_one_to_64_bytes_and_errors_have_fixed_text() {
    let label = |provider: String| {
        let (driver, _) = MemoryDriver::isolated();
        let observer = BoundedMailObserver::new(1).expect("observer");
        ObservedMailDriver::try_new(provider, driver, observer).map(|_| ())
    };
    assert!(label("p".repeat(64)).is_ok());
    assert!(label("p".repeat(65)).is_err());
    assert!(label(String::new()).is_err());
    assert_eq!(
        MailObservationError::InvalidCapacity.to_string(),
        "invalid mail observation capacity"
    );
    assert_eq!(
        MailObservationError::StateUnavailable.to_string(),
        "mail observation state unavailable"
    );
}
