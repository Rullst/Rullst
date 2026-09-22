#![cfg(feature = "schedules-postgres")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(feature = "sqlite")]
#[path = "recurring/broker.rs"]
mod broker;
#[path = "recurring/failures.rs"]
mod failures;
#[path = "recurring/lifecycle.rs"]
mod lifecycle;
#[path = "recurring/support.rs"]
mod support;
use support::*;

#[test]
fn public_inputs_are_bounded_and_deserialization_cannot_bypass_validation() {
    let clock = ManualClock::new();
    for cron in ["* * * * * *", "bad cron", "* * * * 0", "0 0 31 2 *"] {
        assert!(
            RecurringDefinition::new(
                "one",
                cron,
                1_800_000_000_000,
                MissedRunPolicy::CatchUp,
                ScheduledMessage::new("jobs", "ready", []).unwrap()
            )
            .is_err()
        );
    }
    assert!(ScheduledMessage::new("jobs", "ready", vec![0; 16385]).is_err());
    assert!(RecurringConfig::new("one", 0, 1).is_err());
    assert!(config("one").with_lease(Duration::from_secs(301)).is_err());
    let definition = definition("one", &clock, MissedRunPolicy::CatchUp);
    let mut encoded = serde_json::to_value(&definition).unwrap();
    encoded["name"] = serde_json::json!("bad/name");
    assert!(serde_json::from_value::<RecurringDefinition>(encoded).is_err());
    let mut message =
        serde_json::to_value(ScheduledMessage::new("jobs", "ready", []).unwrap()).unwrap();
    message["topic"] = serde_json::json!("bad topic");
    assert!(serde_json::from_value::<ScheduledMessage>(message).is_err());
    assert!(!format!("{definition:?}").contains("PRIVATE"));
}
#[tokio::test]
#[ignore = "requires wrapper-owned PostgreSQL"]
async fn postgres_recurring_contract() {
    let url = url();
    lifecycle::run(&url).await;
    failures::run(&url).await;
    #[cfg(feature = "sqlite")]
    broker::run(&url).await;
}
