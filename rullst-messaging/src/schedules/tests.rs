//! Recurring-store logic that needs no PostgreSQL server: configuration,
//! definitions, metadata accessors, trusted time and input validation.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::{MessageId, MessagingStorageKey, PublishReceipt};
use std::time::Duration;
use zeroize::Zeroizing;

#[derive(Clone)]
struct FixedClock(i64);

impl Clock for FixedClock {
    fn now_millis(&self) -> crate::Result<i64> {
        Ok(self.0)
    }
}

fn config() -> RecurringConfig {
    RecurringConfig::new("school", 10, 100).unwrap()
}

fn message() -> ScheduledMessage {
    ScheduledMessage::new("reports", "report.due", b"{}".to_vec()).unwrap()
}

#[test]
fn trusted_time_ends_eight_days_before_the_last_representable_second() {
    assert_eq!(
        current(&FixedClock(1_800_000_000_000)),
        Ok(1_800_000_000_000)
    );
    assert_eq!(
        current(&FixedClock(253_401_609_599_000)),
        Ok(253_401_609_599_000)
    );
    assert_eq!(
        current(&FixedClock(253_401_609_599_001)),
        Err(RecurringError::Clock)
    );
    assert_eq!(current(&FixedClock(-1)), Err(RecurringError::Clock));
}

#[test]
fn delivery_windows_span_two_leases_to_seven_days() {
    for millis in [120_000, 604_800_000] {
        let config = config()
            .with_delivery_window(Duration::from_millis(millis))
            .unwrap();
        assert_eq!(config.window_ms, millis as i64);
    }
    for millis in [119_999, 604_800_001] {
        assert_eq!(
            config()
                .with_delivery_window(Duration::from_millis(millis))
                .err(),
            Some(RecurringError::InvalidInput("delivery window")),
            "{millis}"
        );
    }
}

#[test]
fn a_lease_fits_twice_into_the_delivery_window() {
    let narrow = || {
        config()
            .with_delivery_window(Duration::from_millis(120_000))
            .unwrap()
    };
    let leased = narrow().with_lease(Duration::from_millis(60_000)).unwrap();
    assert_eq!(leased.lease_ms, 60_000);
    assert_eq!(
        narrow().with_lease(Duration::from_millis(60_001)).err(),
        Some(RecurringError::InvalidInput("lease"))
    );
}

#[test]
fn configuration_reports_its_namespace_binding_and_limits() {
    let config = config();
    assert_eq!(config.namespace(), "school");
    assert_eq!(
        config.binding(),
        b"recurring-v1\nschool\n10\n100\n60000\n86400000".to_vec()
    );
    assert_eq!(
        format!("{config:?}"),
        "RecurringConfig { schedules: 10, occurrences: 100, lease_ms: 60000, \
         delivery_window_ms: 86400000, .. }"
    );
}

#[test]
fn messages_carry_at_most_16_kib_of_payload() {
    assert!(ScheduledMessage::new("reports", "report.due", vec![b'x'; 16_384]).is_ok());
    assert_eq!(
        ScheduledMessage::new("reports", "report.due", vec![b'x'; 16_385]).err(),
        Some(RecurringError::InvalidInput("payload size"))
    );
    assert_eq!(format!("{:?}", message()), "ScheduledMessage([REDACTED])");
}

#[test]
fn definitions_report_their_normalized_fields() {
    let definition = RecurringDefinition::new(
        "daily-report",
        " 0   6 * * * ",
        1_800_000_000_000,
        MissedRunPolicy::Coalesce,
        message(),
    )
    .unwrap();
    assert_eq!(definition.name(), "daily-report");
    assert_eq!(definition.cron_expression(), "0 6 * * *");
    assert_eq!(definition.first_after_ms(), 1_800_000_000_000);
    assert_eq!(definition.missed_run_policy(), MissedRunPolicy::Coalesce);
    assert_eq!(format!("{definition:?}"), "RecurringDefinition([REDACTED])");
}

#[test]
fn cron_expressions_have_five_fields_and_at_most_128_bytes() {
    let define = |expression: &str| {
        RecurringDefinition::new("a", expression, 0, MissedRunPolicy::CatchUp, message())
    };
    let minutes = (0..=42)
        .map(|minute| minute.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let longest = format!("{minutes} 0 10 10 *");
    assert_eq!(longest.len(), 128);
    assert!(define(&longest).is_ok());
    let cron = Some(RecurringError::InvalidInput("five-field UTC cron"));
    let longer = format!("{minutes} 10 10 10 *");
    assert_eq!(longer.len(), 129);
    assert_eq!(define(&longer).err(), cron);
    assert_eq!(define("0 6 * *").err(), cron);
}

#[test]
fn metadata_accessors_return_their_fields() {
    let occurrence = OccurrenceMetadata {
        id: "f".repeat(64),
        schedule: "daily-report".to_owned(),
        due: 1_800_000_000_000,
        created: 1_800_000_001_000,
        expires: 1_800_086_401_000,
        attempts: 3,
        state: OccurrenceState::Leased,
    };
    assert_eq!(occurrence.id(), "f".repeat(64));
    assert_eq!(occurrence.schedule_name(), "daily-report");
    assert_eq!(occurrence.due_at_ms(), 1_800_000_000_000);
    assert_eq!(occurrence.created_at_ms(), 1_800_000_001_000);
    assert_eq!(occurrence.expires_at_ms(), 1_800_086_401_000);
    assert_eq!(occurrence.attempts(), 3);
    assert_eq!(occurrence.state(), OccurrenceState::Leased);

    let lease = OccurrenceLease {
        namespace: "school".to_owned(),
        token: Zeroizing::new("token".to_owned()),
        version: 2,
        expires: 1_800_000_061_000,
        metadata: occurrence.clone(),
    };
    assert_eq!(lease.lease_expires_at_ms(), 1_800_000_061_000);
    assert_eq!(lease.metadata(), &occurrence);
    assert_eq!(format!("{lease:?}"), "OccurrenceLease([REDACTED])");

    let active = RecurringMetadata {
        name: "daily-report".to_owned(),
        generation: "e".repeat(64),
        created: 1_800_000_000_000,
        next_due: Some(1_800_021_600_000),
        cancelled: false,
    };
    assert_eq!(active.name(), "daily-report");
    assert_eq!(active.generation(), "e".repeat(64));
    assert_eq!(active.created_at_ms(), 1_800_000_000_000);
    assert_eq!(active.next_due_at_ms(), Some(1_800_021_600_000));
    assert!(!active.is_cancelled());
    let cancelled = RecurringMetadata {
        next_due: None,
        cancelled: true,
        ..active
    };
    assert_eq!(cancelled.next_due_at_ms(), None);
    assert!(cancelled.is_cancelled());

    let publication = PublishReceipt::new(
        MessageId::from_stored("msg_0123456789abcdef0123456789abcdef".to_owned()).unwrap(),
        false,
        1_800_000_000_000,
    );
    for acknowledged in [false, true] {
        let receipt = RecurringRelayReceipt {
            publication: publication.clone(),
            acknowledged,
        };
        assert_eq!(receipt.occurrence_acknowledged(), acknowledged);
        assert_eq!(receipt.publication(), &publication);
    }
}

#[test]
fn identifiers_and_batches_are_validated_locally() {
    let valid = "0123456789abcdef".repeat(4);
    assert_eq!(inspection::id(&valid), Ok(()));
    let invalid = Err(RecurringError::InvalidInput("occurrence ID"));
    assert_eq!(inspection::id(&valid[..63]), invalid);
    assert_eq!(inspection::id(&format!("{}g", &valid[..63])), invalid);
    assert_eq!(inspection::id(&valid.to_uppercase()), invalid);
    assert_eq!(tick::batch(1), Ok(1));
    assert_eq!(tick::batch(100), Ok(100));
    for limit in [0, 101] {
        assert_eq!(
            tick::batch(limit),
            Err(RecurringError::InvalidInput("batch limit"))
        );
    }
}

/// A store whose pool points at a socket that does not exist: every request
/// that reaches storage fails at once with `Storage`.
fn unreachable_store() -> PostgresRecurringStore<FixedClock> {
    let options = sqlx::postgres::PgConnectOptions::new()
        .socket("/nonexistent/rullst-recurring-unit-tests")
        .database("none");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(options);
    PostgresRecurringStore {
        pool,
        config: config(),
        keys: Arc::new(MessagingKeyring::new(
            MessagingStorageKey::try_new("k", [9; 32]).unwrap(),
        )),
        clock: FixedClock(1_800_000_000_000),
    }
}

#[tokio::test]
async fn invalid_requests_are_refused_before_storage_is_contacted() {
    let store = unreachable_store();
    let batch = Some(RecurringError::InvalidInput("batch limit"));
    let name = Some(RecurringError::InvalidInput("schedule name"));
    assert_eq!(store.tick(0).await.err(), batch);
    assert_eq!(store.claim(0).await.err(), batch);
    assert_eq!(store.schedules(None, 0).await.err(), batch);
    assert_eq!(store.schedules(Some("bad name"), 1).await.err(), name);
    assert_eq!(store.occurrences(None, 101).await.err(), batch);
    assert_eq!(
        store.retry_failed("not-an-id").await.err(),
        Some(RecurringError::InvalidInput("occurrence ID"))
    );
    assert_eq!(store.purge_terminal(1, 0).await.err(), batch);
    assert_eq!(store.cancel("bad name").await.err(), name);
    // A valid request does reach the (missing) storage.
    assert_eq!(
        store.cancel("daily-report").await.err(),
        Some(RecurringError::Storage)
    );
}
