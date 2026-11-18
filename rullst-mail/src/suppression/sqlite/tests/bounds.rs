//! Quota, replay, stored-row and target checks of the SQLite store.

use super::*;

async fn tamper(url: &str, statement: &'static str) {
    let pool = SqlitePool::connect(url).await.expect("open fixture");
    let mut connection = pool.acquire().await.expect("fixture connection");
    // The schema's CHECK constraints would refuse the corrupt values.
    sqlx::query("PRAGMA ignore_check_constraints = ON")
        .execute(&mut *connection)
        .await
        .expect("disable checks");
    sqlx::query(statement)
        .execute(&mut *connection)
        .await
        .expect("tamper fixture");
    drop(connection);
    pool.close().await;
}

fn event_at(id: &str, recipient: &str, reason: SuppressionReason, at: u64) -> SuppressionEvent {
    SuppressionEvent::try_new("fixture", id, recipient, reason, at).expect("valid event")
}

#[test]
fn every_volatile_database_form_is_refused() {
    let volatile = |url: &str| {
        let options = SqliteConnectOptions::from_str(url).expect("parse URL");
        volatile_database_url(url, options.get_filename())
    };
    // An empty name, `:memory:` spelled as a URL or percent-encoded, and
    // `mode=memory` each select a database that is never written to disk.
    for url in [
        "sqlite://",
        "sqlite://:memory:",
        "sqlite://%3Amemory%3A",
        "sqlite:///tmp/mail.sqlite?mode=memory",
    ] {
        assert!(volatile(url), "{url}");
    }
    assert!(!volatile("sqlite:///tmp/mail.sqlite?mode=rwc"));
}

#[tokio::test]
async fn each_quota_applies_alone_and_replays_must_match_exactly() {
    let path = temporary_database("bounds");
    let url = database_url(&path);
    let timestamp = now();

    // Two events for one recipient exhaust the event quota alone.
    let events = SqliteSuppressionStore::connect(&url, 4, 2).await.unwrap();
    for (index, id) in ["e1", "e2"].into_iter().enumerate() {
        let at = timestamp + index as u64;
        let reason = SuppressionReason::HardBounce;
        events
            .record(event_at(id, "alice@example.com", reason, at))
            .await
            .unwrap();
    }
    let third = event_at(
        "e3",
        "alice@example.com",
        SuppressionReason::HardBounce,
        timestamp,
    );
    assert_eq!(
        events.record(third).await,
        Err(SuppressionError::CapacityExceeded)
    );

    // A later event leaves the first and last times apart.
    let record = events.lookup("alice@example.com").await.unwrap().unwrap();
    assert_eq!(
        (record.first_seen_at(), record.last_seen_at()),
        (timestamp, timestamp + 1)
    );

    // A replay must repeat the reason and the observation time too.
    for replay in [
        event_at(
            "e1",
            "alice@example.com",
            SuppressionReason::HardBounce,
            timestamp + 5,
        ),
        event_at(
            "e1",
            "alice@example.com",
            SuppressionReason::SpamComplaint,
            timestamp,
        ),
    ] {
        assert_eq!(
            events.record(replay).await,
            Err(SuppressionError::EventConflict)
        );
    }

    // A closed store's clones see the closed pool.
    let clone = events.clone();
    events.close().await;
    assert!(matches!(
        clone.snapshot().await,
        Err(SuppressionError::StorageUnavailable(_))
    ));
    remove_database(&path);

    // One suppressed recipient exhausts the recipient quota alone.
    let path = temporary_database("recipients");
    let recipients = SqliteSuppressionStore::connect(database_url(&path), 1, 4)
        .await
        .unwrap();
    let bob = event_at(
        "b1",
        "bob@example.com",
        SuppressionReason::Manual,
        timestamp,
    );
    recipients.record(bob).await.unwrap();
    let carol = event_at(
        "c1",
        "carol@example.com",
        SuppressionReason::Manual,
        timestamp,
    );
    assert_eq!(
        recipients.record(carol).await,
        Err(SuppressionError::CapacityExceeded)
    );
    recipients.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn corrupt_stored_times_and_limits_fail_closed() {
    let path = temporary_database("stored");
    let url = database_url(&path);
    let store = SqliteSuppressionStore::connect(&url, 4, 4).await.unwrap();
    let timestamp = now();
    let event = event_at(
        "t1",
        "dave@example.com",
        SuppressionReason::Manual,
        timestamp,
    );
    store.record(event).await.unwrap();
    store.close().await;

    let corrupt = Err(SuppressionError::CorruptStorage("recipient timestamps"));
    for statement in [
        "UPDATE rullst_mail_suppressed_recipients SET first_seen_at = 0",
        "UPDATE rullst_mail_suppressed_recipients SET first_seen_at = last_seen_at + 1",
    ] {
        tamper(&url, statement).await;
        let store = SqliteSuppressionStore::connect(&url, 4, 4).await.unwrap();
        assert_eq!(
            store.lookup("dave@example.com").await,
            corrupt,
            "{statement}"
        );
        store.close().await;
    }

    tamper(
        &url,
        "UPDATE rullst_mail_suppression_meta SET max_events = 0",
    )
    .await;
    assert!(matches!(
        SqliteSuppressionStore::connect(&url, 4, 4).await,
        Err(SuppressionError::CorruptStorage("schema configuration"))
    ));
    remove_database(&path);
}

#[tokio::test]
async fn only_a_missing_or_regular_file_target_is_opened() {
    let directory = temporary_database("directory");
    std::fs::create_dir(&directory).expect("create directory target");
    assert!(matches!(
        SqliteSuppressionStore::connect(database_url(&directory), 4, 4).await,
        Err(SuppressionError::InvalidConfiguration(
            "target must be a regular file"
        ))
    ));
    std::fs::remove_dir(&directory).expect("remove directory target");

    // Any inspection error other than NotFound refuses the target. A NUL byte
    // is refused as InvalidInput on every platform.
    assert!(matches!(
        super::super::reject_existing_unsafe_target(Path::new("bad\0name.sqlite")),
        Err(SuppressionError::StorageUnavailable(
            "inspect database target"
        ))
    ));

    // A path below a regular file: Unix inspection fails with ENOTDIR, not
    // NotFound. Windows reports it as not found, and SQLite cannot create it.
    let file = temporary_database("parent-file");
    std::fs::File::create(&file).expect("create parent file");
    let below_file =
        SqliteSuppressionStore::connect(database_url(&file.join("mail.sqlite")), 4, 4).await;
    #[cfg(unix)]
    assert!(matches!(
        below_file,
        Err(SuppressionError::StorageUnavailable(
            "inspect database target"
        ))
    ));
    #[cfg(not(unix))]
    assert!(matches!(
        below_file,
        Err(SuppressionError::StorageUnavailable("connect database"))
    ));
    std::fs::remove_file(&file).expect("remove parent file");
}
