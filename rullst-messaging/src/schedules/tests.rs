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
        syntax: CronSyntax::Posix,
    };
    assert_eq!(active.name(), "daily-report");
    assert_eq!(active.generation(), "e".repeat(64));
    assert_eq!(active.created_at_ms(), 1_800_000_000_000);
    assert_eq!(active.next_due_at_ms(), Some(1_800_021_600_000));
    assert!(!active.is_cancelled());
    assert_eq!(active.syntax(), CronSyntax::Posix);
    let cancelled = RecurringMetadata {
        next_due: None,
        cancelled: true,
        syntax: CronSyntax::Legacy,
        ..active
    };
    assert_eq!(cancelled.next_due_at_ms(), None);
    assert!(cancelled.is_cancelled());
    assert_eq!(cancelled.syntax(), CronSyntax::Legacy);

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

/// 2026-11-17T10:14:30Z, a Tuesday.
const TUESDAY: i64 = 1_794_910_470_000;

fn posix(expression: &str) -> RecurringDefinition {
    RecurringDefinition::new("weekly", expression, 0, MissedRunPolicy::CatchUp, message()).unwrap()
}

/// A definition as stored before 13.0.0-alpha.6, without a syntax marker.
fn legacy(expression: &str) -> RecurringDefinition {
    let mut stored = serde_json::to_value(posix("0 0 1 1 *")).unwrap();
    let object = stored.as_object_mut().unwrap();
    assert_eq!(object.remove("syntax"), Some(serde_json::json!("posix")));
    object.insert("expression".into(), expression.into());
    serde_json::from_value(stored).unwrap()
}

#[test]
fn new_definitions_read_posix_cron_like_core() {
    let definition = posix("0 9 * * 1");
    assert_eq!(definition.syntax(), CronSyntax::Posix);
    // Weekday 1 is Monday; 0 and 7 are Sunday.
    assert_eq!(definition.next_after(TUESDAY), Ok(Some(1_795_424_400_000)));
    assert_eq!(
        posix("0 9 * * 0").next_after(TUESDAY),
        Ok(Some(1_795_338_000_000))
    );
    assert_eq!(
        posix("0 9 * * 7").next_after(TUESDAY),
        Ok(Some(1_795_338_000_000))
    );
    // Both day fields restricted: the 1st of the month or a Monday.
    assert_eq!(
        posix("0 0 1 * 1").next_after(TUESDAY),
        Ok(Some(1_795_392_000_000))
    );
    let define = |expression: &str| {
        RecurringDefinition::new("a", expression, 0, MissedRunPolicy::CatchUp, message())
    };
    assert_eq!(
        define("1, 2 * * *").err(),
        Some(RecurringError::InvalidInput(
            "POSIX five-field UTC cron; this expression uses the legacy syntax"
        ))
    );
    assert_eq!(
        define("0 0 * * 8").err(),
        Some(RecurringError::InvalidInput("five-field UTC cron"))
    );
    assert_eq!(
        define("0 0 31 2 *").err(),
        Some(RecurringError::InvalidInput("exhausted cron"))
    );
}

#[test]
fn stored_definitions_without_a_marker_keep_the_legacy_grammar() {
    let stored = legacy("0 9 * * 1");
    assert_eq!(stored.syntax(), CronSyntax::Legacy);
    // Weekday 1 was Sunday.
    assert_eq!(stored.next_after(TUESDAY), Ok(Some(1_795_338_000_000)));
    // Joined tokens keep running.
    assert_eq!(
        legacy("1, 2 * * *").next_after(TUESDAY),
        Ok(Some(1_794_913_260_000))
    );
    // A legacy definition is stored again without the marker, a POSIX one
    // with it, and an unknown syntax is refused.
    let json = serde_json::to_value(&stored).unwrap();
    assert!(json.get("syntax").is_none());
    assert_eq!(
        serde_json::from_value::<RecurringDefinition>(json).unwrap(),
        stored
    );
    let json = serde_json::to_value(posix("0 9 * * 1")).unwrap();
    assert_eq!(json["syntax"], "posix");
    let mut unknown = json;
    unknown["syntax"] = "quartz".into();
    assert!(serde_json::from_value::<RecurringDefinition>(unknown).is_err());
}

#[test]
fn create_replays_a_legacy_definition_only_with_the_same_fields() {
    let stored = legacy("0 9 * * 1");
    // Equal definitions replay quietly.
    assert_eq!(stored.replays(&stored.clone()), Ok(false));
    assert_eq!(posix("0 9 * * 1").replays(&posix("0 9 * * 1")), Ok(false));
    // The same expression and fields keep the legacy schedule, with a warning.
    assert_eq!(stored.replays(&posix("0 9 * * 1")), Ok(true));
    // Anything else on a legacy name, the POSIX rewrite included, is refused.
    for requested in [
        posix("0 9 * * 0"),
        RecurringDefinition::new(
            "weekly",
            "0 9 * * 1",
            60_000,
            MissedRunPolicy::CatchUp,
            message(),
        )
        .unwrap(),
        RecurringDefinition::new(
            "weekly",
            "0 9 * * 1",
            0,
            MissedRunPolicy::Coalesce,
            message(),
        )
        .unwrap(),
        RecurringDefinition::new(
            "weekly",
            "0 9 * * 1",
            0,
            MissedRunPolicy::CatchUp,
            ScheduledMessage::new("reports", "report.other", b"{}".to_vec()).unwrap(),
        )
        .unwrap(),
    ] {
        assert_eq!(
            stored.replays(&requested),
            Err(RecurringError::LegacyDefinition)
        );
    }
    let mut renamed = posix("0 9 * * 1");
    renamed.name = "other".into();
    assert_eq!(
        stored.replays(&renamed),
        Err(RecurringError::LegacyDefinition)
    );
    // A POSIX definition conflicts as before, even with a legacy request.
    assert_eq!(
        posix("0 9 * * 1").replays(&posix("0 9 * * 0")),
        Err(RecurringError::Conflict)
    );
    assert_eq!(
        posix("0 9 * * 1").replays(&stored),
        Err(RecurringError::Conflict)
    );
    assert!(
        RecurringError::LegacyDefinition
            .to_string()
            .contains("keep its expression to keep it running")
    );
}
