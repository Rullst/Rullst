//! Quotas and reason/provider merging of the process-local store.

use super::*;
use crate::suppression::memory::merge_record;

#[tokio::test]
async fn each_process_local_quota_applies_alone() {
    let at = unix_time().expect("clock");
    let manual = SuppressionReason::Manual;
    // Two events for one recipient exhaust the event quota alone.
    let events = InMemorySuppressionStore::new(4, 2).expect("bounded store");
    for id in ["e1", "e2"] {
        let accepted = event("fixture", id, "alice@example.com", manual, at);
        events.record(accepted).await.expect("within quota");
    }
    let third = event("fixture", "e3", "alice@example.com", manual, at);
    assert_eq!(
        events.record(third).await,
        Err(SuppressionError::CapacityExceeded)
    );
    // One suppressed recipient exhausts the recipient quota alone.
    let recipients = InMemorySuppressionStore::new(1, 4).expect("bounded store");
    let bob = event("fixture", "b1", "bob@example.com", manual, at);
    recipients.record(bob).await.expect("within quota");
    let carol = event("fixture", "c1", "carol@example.com", manual, at);
    assert_eq!(
        recipients.record(carol).await,
        Err(SuppressionError::CapacityExceeded)
    );
}

#[test]
fn the_strongest_then_newest_event_names_the_provider() {
    let at = unix_time().expect("clock") - 100;
    let bounce = SuppressionReason::HardBounce;
    let recipient = "alice@example.com";
    let first = merge_record(None, &event("first", "a", recipient, bounce, at + 10));

    // An older event of the same reason keeps the newer provider.
    let older = merge_record(Some(&first), &event("older", "b", recipient, bounce, at));
    assert_eq!(older.provider(), "first");
    assert_eq!((older.first_seen_at(), older.last_seen_at()), (at, at + 10));

    // A newer event of the same reason names its provider.
    let newer = merge_record(
        Some(&first),
        &event("newer", "c", recipient, bounce, at + 20),
    );
    assert_eq!(newer.provider(), "newer");

    // A newer but weaker reason never downgrades the record.
    let manual = SuppressionReason::Manual;
    let weaker = merge_record(
        Some(&first),
        &event("weaker", "d", recipient, manual, at + 20),
    );
    assert_eq!((weaker.reason(), weaker.provider()), (bounce, "first"));
}
