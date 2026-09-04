use super::*;

#[test]
fn windows_file_url_target_removes_only_a_leading_drive_separator() {
    assert_eq!(
        windows_file_url_target("/C:/temp/mail.sqlite"),
        Some("C:/temp/mail.sqlite")
    );
    assert_eq!(
        windows_file_url_target("\\D:/temp/mail.sqlite"),
        Some("D:/temp/mail.sqlite")
    );
    assert_eq!(windows_file_url_target("C:/temp/mail.sqlite"), None);
    assert_eq!(windows_file_url_target("/tmp/mail.sqlite"), None);
    assert_eq!(windows_file_url_target("//server/share/mail.sqlite"), None);
}
use crate::drivers::MemoryDriver;
use crate::{MailDriver, MailError, Message, SuppressionGuard};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_secs()
}

fn event(index: usize, recipient: &str, reason: SuppressionReason) -> SuppressionEvent {
    SuppressionEvent::try_new(
        "fixture",
        format!("event-{index}"),
        recipient,
        reason,
        now(),
    )
    .expect("valid event")
}

fn temporary_database(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-mail-suppression-{label}-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

fn database_url(path: &Path) -> String {
    format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"))
}

fn remove_database(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let candidate = PathBuf::from(format!("{}{suffix}", path.display()));
        match std::fs::remove_file(candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove SQLite fixture: {error}"),
        }
    }
}

#[tokio::test]
// TM-MAIL-02: a durable suppression still blocks after restart and before transport.
async fn suppression_guard_observes_restart_safe_state_before_transport() {
    let path = temporary_database("restart");
    let url = database_url(&path);
    let store = SqliteSuppressionStore::connect(&url, 32, 64)
        .await
        .expect("connect store");
    store
        .record(event(
            1,
            "blocked@example.com",
            SuppressionReason::HardBounce,
        ))
        .await
        .expect("record bounce");
    store.close().await;

    let reopened = SqliteSuppressionStore::connect(&url, 32, 64)
        .await
        .expect("reopen store");
    let (driver, deliveries) = MemoryDriver::isolated();
    let guard = SuppressionGuard::new(driver, reopened.clone());
    let message = Message::new()
        .to("blocked@EXAMPLE.com")
        .subject("blocked")
        .text("must not leave");
    assert_eq!(
        guard.send(&message).await,
        Err(MailError::SuppressedRecipient {
            reason: "hard_bounce"
        })
    );
    assert!(deliveries.lock().expect("deliveries").is_empty());
    assert_eq!(reopened.snapshot().await.expect("snapshot").recipients(), 1);
    reopened.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn two_instances_enforce_exact_transactional_quotas() {
    let path = temporary_database("quota");
    let url = database_url(&path);
    let first = SqliteSuppressionStore::connect(&url, 4, 4)
        .await
        .expect("first store");
    let second = SqliteSuppressionStore::connect(&url, 4, 4)
        .await
        .expect("second store");
    let handles = (0..8)
        .map(|index| {
            let store = if index % 2 == 0 {
                first.clone()
            } else {
                second.clone()
            };
            tokio::spawn(async move {
                store
                    .record(event(
                        index,
                        &format!("recipient-{index}@example.com"),
                        SuppressionReason::HardBounce,
                    ))
                    .await
            })
        })
        .collect::<Vec<_>>();
    let mut accepted = 0;
    let mut exhausted = 0;
    for handle in handles {
        match handle.await.expect("event task") {
            Ok(_) => accepted += 1,
            Err(SuppressionError::CapacityExceeded) => exhausted += 1,
            Err(error) => panic!("unexpected suppression error: {error}"),
        }
    }
    assert_eq!((accepted, exhausted), (4, 4));
    assert_eq!(
        first.snapshot().await.expect("snapshot"),
        SuppressionSnapshot::new(4, 4, 4, 4)
    );
    first.close().await;
    second.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn replay_conflicts_reason_escalation_pruning_and_config_drift_are_bounded() {
    let path = temporary_database("events");
    let url = database_url(&path);
    let store = SqliteSuppressionStore::connect(&url, 4, 4)
        .await
        .expect("connect store");
    let timestamp = now();
    let original = SuppressionEvent::try_new(
        "postmark",
        "event-shared",
        "alice@example.com",
        SuppressionReason::Manual,
        timestamp,
    )
    .expect("manual event");
    let first = store.record(original.clone()).await.expect("first event");
    assert_eq!(store.record(original).await.expect("exact replay"), first);
    let conflict = SuppressionEvent::try_new(
        "postmark",
        "event-shared",
        "bob@example.com",
        SuppressionReason::Manual,
        timestamp,
    )
    .expect("conflicting event shape");
    assert_eq!(
        store.record(conflict).await,
        Err(SuppressionError::EventConflict)
    );
    let complaint = SuppressionEvent::try_new(
        "sendgrid",
        "event-complaint",
        "alice@example.com",
        SuppressionReason::SpamComplaint,
        timestamp,
    )
    .expect("complaint");
    let escalated = store.record(complaint).await.expect("escalate");
    assert_eq!(escalated.reason(), SuppressionReason::SpamComplaint);
    assert_eq!(escalated.provider(), "sendgrid");
    assert_eq!(store.prune_events_before(timestamp + 1).await.unwrap(), 2);
    assert_eq!(store.snapshot().await.unwrap().events(), 0);
    assert!(store.lookup("alice@example.com").await.unwrap().is_some());
    store.close().await;

    assert!(matches!(
        SqliteSuppressionStore::connect(&url, 5, 4).await,
        Err(SuppressionError::InvalidConfiguration(
            "limits conflict with stored configuration"
        ))
    ));
    remove_database(&path);
}

#[cfg(unix)]
#[tokio::test]
async fn memory_symlink_and_corrupt_schema_targets_fail_closed() {
    use std::os::unix::fs::symlink;

    assert!(matches!(
        SqliteSuppressionStore::connect("sqlite::memory:", 4, 4).await,
        Err(SuppressionError::InvalidConfiguration(
            "database must be file-backed"
        ))
    ));
    let target = temporary_database("target");
    let link = temporary_database("link");
    std::fs::File::create(&target).expect("create target");
    symlink(&target, &link).expect("create symlink");
    assert!(matches!(
        SqliteSuppressionStore::connect(database_url(&link), 4, 4).await,
        Err(SuppressionError::InvalidConfiguration(
            "target must be a regular file"
        ))
    ));
    std::fs::remove_file(link).expect("remove link");
    std::fs::remove_file(target).expect("remove target");

    let corrupt = temporary_database("corrupt");
    let url = database_url(&corrupt);
    let store = SqliteSuppressionStore::connect(&url, 4, 4)
        .await
        .expect("create store");
    store.close().await;
    let pool = SqlitePool::connect(&url).await.expect("open fixture");
    sqlx::query("UPDATE rullst_mail_suppression_meta SET schema_version = 2 WHERE id = 1")
        .execute(&pool)
        .await
        .expect("corrupt version");
    pool.close().await;
    assert!(matches!(
        SqliteSuppressionStore::connect(&url, 4, 4).await,
        Err(SuppressionError::CorruptStorage("schema configuration"))
    ));
    remove_database(&corrupt);
}
