use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn sqlite_url(label: &str) -> (String, std::path::PathBuf) {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "rullst-capital-webhook-{label}-{}-{sequence}.sqlite",
        std::process::id()
    ));
    let portable_path = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    let url = format!("sqlite:///{portable_path}?mode=rwc");
    #[cfg(not(windows))]
    let url = format!("sqlite://{portable_path}?mode=rwc");
    (url, path)
}

async fn record_at(
    store: &SqlWebhookReplayStore,
    provider: &str,
    payload: &[u8],
    accepted_at: u64,
) -> Result<(), CapitalError> {
    let mut transaction = store
        .pool
        .begin()
        .await
        .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
    let result = store
        .check_and_record_at(&mut transaction, provider, payload, accepted_at)
        .await;
    finish_transaction(transaction, result).await
}

#[test]
fn validates_profiles_and_backend_urls() {
    assert!(validate_profile(1, Duration::from_secs(1)).is_ok());
    assert!(validate_profile(0, Duration::from_secs(1)).is_err());
    assert!(
        validate_profile(
            super::super::MAX_REPLAY_CAPACITY + 1,
            Duration::from_secs(1)
        )
        .is_err()
    );
    assert!(validate_profile(1, Duration::ZERO).is_err());
    assert!(validate_profile(1, super::super::MAX_REPLAY_TTL + Duration::from_secs(1)).is_err());
    assert_eq!(
        backend_from_url("postgres://localhost/db"),
        Ok(SqlWebhookBackend::Postgres)
    );
    assert_eq!(
        backend_from_url("mysql://localhost/db"),
        Ok(SqlWebhookBackend::Mysql)
    );
    assert_eq!(
        backend_from_url("sqlite::memory:"),
        Ok(SqlWebhookBackend::Sqlite)
    );
    assert!(backend_from_url("mongodb://localhost/db").is_err());
    assert_eq!(
        timestamp_sql(SqlWebhookBackend::Postgres),
        "SELECT CAST(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP) AS BIGINT)"
    );
    assert_eq!(
        timestamp_sql(SqlWebhookBackend::Mysql),
        "SELECT UNIX_TIMESTAMP()"
    );
    assert_eq!(
        timestamp_sql(SqlWebhookBackend::Sqlite),
        "SELECT CAST(strftime('%s', 'now') AS INTEGER)"
    );
}

#[tokio::test]
async fn sqlite_claims_survive_restart_and_fail_closed_at_capacity() {
    let (url, path) = sqlite_url("restart");
    let first = SqlWebhookReplayStore::connect(&url, 2, Duration::from_secs(60))
        .await
        .expect("first SQLite replay store");
    first.prepare_schema().await.expect("replay schema");
    first
        .check_and_record_payload("stripe", b"event-one")
        .await
        .expect("first claim");
    first.close().await;

    let reopened = SqlWebhookReplayStore::connect(&url, 2, Duration::from_secs(60))
        .await
        .expect("reopened SQLite replay store");
    reopened.prepare_schema().await.expect("existing schema");
    assert!(matches!(
        reopened
            .check_and_record_payload("stripe", b"event-one")
            .await,
        Err(CapitalError::WebhookReplay(_))
    ));
    reopened
        .check_and_record_payload("stripe", b"event-two")
        .await
        .expect("second claim");
    assert_eq!(
        reopened
            .check_and_record_payload("stripe", b"event-three")
            .await,
        Err(CapitalError::WebhookReplayStoreFull)
    );
    reopened.close().await;
    std::fs::remove_file(path).expect("remove closed SQLite fixture");
}

#[tokio::test]
async fn in_memory_sqlite_keeps_its_only_connection() {
    let store = SqlWebhookReplayStore::connect("sqlite::memory:", 1, Duration::from_secs(10))
        .await
        .expect("SQLite replay store");
    let options = store.pool().options();
    assert_eq!(options.get_max_connections(), 1);
    assert_eq!(options.get_min_connections(), 1);
    assert_eq!(options.get_idle_timeout(), None);
    assert_eq!(options.get_max_lifetime(), None);
    store.close().await;
}

#[tokio::test]
async fn sqlite_expiry_and_configuration_drift_are_explicit() {
    let store = SqlWebhookReplayStore::connect("sqlite::memory:", 1, Duration::from_secs(10))
        .await
        .expect("SQLite replay store");
    store.prepare_schema().await.expect("replay schema");
    record_at(&store, "stripe", b"same-event", 100)
        .await
        .expect("initial deterministic claim");
    assert!(matches!(
        record_at(&store, "stripe", b"same-event", 109).await,
        Err(CapitalError::WebhookReplay(_))
    ));
    record_at(&store, "stripe", b"same-event", 110)
        .await
        .expect("claim after exact TTL boundary");

    let drifted = SqlWebhookReplayStore::from_pool(
        store.pool.clone(),
        SqlWebhookBackend::Sqlite,
        2,
        Duration::from_secs(10),
    )
    .expect("valid alternate local profile");
    assert_eq!(
        drifted.prepare_schema().await,
        Err(CapitalError::WebhookReplayConfigurationDrift)
    );
    store.close().await;
}

#[tokio::test]
async fn sqlite_concurrent_duplicate_has_one_winner() {
    let (url, path) = sqlite_url("concurrency");
    let store = std::sync::Arc::new(
        SqlWebhookReplayStore::connect(&url, 32, Duration::from_secs(60))
            .await
            .expect("SQLite replay store"),
    );
    store.prepare_schema().await.expect("replay schema");

    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let store = store.clone();
        tasks.spawn(async move {
            store
                .check_and_record_payload("stripe", b"concurrent-event")
                .await
        });
    }
    let mut accepted = 0;
    let mut replayed = 0;
    while let Some(result) = tasks.join_next().await {
        match result.expect("replay task") {
            Ok(()) => accepted += 1,
            Err(CapitalError::WebhookReplay(_)) => replayed += 1,
            Err(error) => panic!("unexpected replay-store error: {error}"),
        }
    }
    assert_eq!(accepted, 1);
    assert_eq!(replayed, 7);

    store.close().await;
    std::fs::remove_file(path).expect("remove closed SQLite fixture");
}

#[tokio::test]
async fn caller_transaction_commits_or_rolls_back_claim_with_domain_effect() {
    let store = SqlWebhookReplayStore::connect("sqlite::memory:", 8, Duration::from_secs(60))
        .await
        .expect("SQLite replay store");
    store.prepare_schema().await.expect("replay schema");
    rullst_orm::sqlx::query("CREATE TABLE webhook_effects (event_name TEXT PRIMARY KEY NOT NULL)")
        .execute(store.pool())
        .await
        .expect("domain fixture schema");

    let mut rolled_back = store.pool().begin().await.expect("rollback transaction");
    store
        .check_and_record_event_key_with_transaction(
            &mut rolled_back,
            "stripe",
            "evt_transaction_1",
        )
        .await
        .expect("transactional claim");
    rullst_orm::sqlx::query("INSERT INTO webhook_effects (event_name) VALUES (?)")
        .bind("rolled-back")
        .execute(&mut *rolled_back)
        .await
        .expect("transactional domain effect");
    rolled_back.rollback().await.expect("explicit rollback");

    let mut committed = store.pool().begin().await.expect("commit transaction");
    store
        .check_and_record_event_key_with_transaction(&mut committed, "stripe", "evt_transaction_1")
        .await
        .expect("claim was rolled back with effect");
    rullst_orm::sqlx::query("INSERT INTO webhook_effects (event_name) VALUES (?)")
        .bind("committed")
        .execute(&mut *committed)
        .await
        .expect("committed domain effect");
    committed.commit().await.expect("atomic commit");

    assert!(matches!(
        store
            .check_and_record_event_key("stripe", "evt_transaction_1")
            .await,
        Err(CapitalError::WebhookReplay(_))
    ));
    let effects = rullst_orm::sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM webhook_effects WHERE event_name = ?",
    )
    .bind("committed")
    .fetch_one(store.pool())
    .await
    .expect("domain effect count");
    assert_eq!(effects, 1);
    store.close().await;
}

#[test]
fn profile_limits_are_inclusive() {
    assert!(validate_profile(1_000_000, Duration::from_secs(1)).is_ok());
    assert!(validate_profile(1_000_001, Duration::from_secs(1)).is_err());
    assert!(validate_profile(1, Duration::from_secs(2_592_000)).is_ok());
    assert!(validate_profile(1, Duration::from_secs(2_592_001)).is_err());
}

#[tokio::test]
async fn sqlite_store_debug_index_clock_bounds_and_close_are_observable() {
    let store = SqlWebhookReplayStore::connect("sqlite::memory:", 4, Duration::from_secs(60))
        .await
        .expect("SQLite replay store");
    let debug = format!("{store:?}");
    assert!(debug.starts_with("SqlWebhookReplayStore"), "{debug}");
    assert!(debug.contains("backend: Sqlite"), "{debug}");
    assert!(debug.contains("max_entries: 4"), "{debug}");
    store.prepare_schema().await.expect("replay schema");
    let indexes = rullst_orm::sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'rullst_webhook_replay_expiry'",
    )
    .fetch_one(store.pool())
    .await
    .expect("index lookup");
    assert_eq!(indexes, 1);

    store
        .check_and_record_payload("stripe", b"database-clock")
        .await
        .expect("claim");
    let accepted_at = rullst_orm::sqlx::query_scalar::<_, i64>(
        "SELECT accepted_at FROM rullst_webhook_replay_claims",
    )
    .fetch_one(store.pool())
    .await
    .expect("claim time");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after the epoch")
        .as_secs() as i64;
    assert!((accepted_at - now).abs() <= 300, "{accepted_at} vs {now}");

    assert!(matches!(
        store.check_and_record_payload("stripe", b"").await,
        Err(CapitalError::ConfigurationError(_))
    ));
    store
        .check_and_record_payload("stripe", &vec![b'x'; 2_097_152])
        .await
        .expect("a 2 MiB payload is within the limit");
    assert!(matches!(
        store
            .check_and_record_payload("stripe", &vec![b'y'; 2_097_153])
            .await,
        Err(CapitalError::ConfigurationError(_))
    ));

    store.close().await;
    assert!(store.pool().is_closed());
}

#[tokio::test]
async fn caller_transaction_records_a_payload_claim() {
    let store = SqlWebhookReplayStore::connect("sqlite::memory:", 4, Duration::from_secs(60))
        .await
        .expect("SQLite replay store");
    store.prepare_schema().await.expect("replay schema");
    let mut transaction = store.pool().begin().await.expect("transaction");
    store
        .check_and_record_payload_with_transaction(&mut transaction, "stripe", b"in-transaction")
        .await
        .expect("transactional payload claim");
    transaction.commit().await.expect("commit");
    assert!(matches!(
        store
            .check_and_record_payload("stripe", b"in-transaction")
            .await,
        Err(CapitalError::WebhookReplay(_))
    ));
    store.close().await;
}

#[tokio::test]
async fn a_failed_claim_insert_that_is_not_a_duplicate_is_unavailable() {
    let store = SqlWebhookReplayStore::connect("sqlite::memory:", 4, Duration::from_secs(60))
        .await
        .expect("SQLite replay store");
    store.prepare_schema().await.expect("replay schema");
    rullst_orm::sqlx::query(
        "CREATE TRIGGER reject_claim BEFORE INSERT ON rullst_webhook_replay_claims BEGIN SELECT RAISE(ABORT, 'injected claim failure'); END",
    )
    .execute(store.pool())
    .await
    .expect("fault trigger");
    assert_eq!(
        store.check_and_record_payload("stripe", b"blocked").await,
        Err(CapitalError::WebhookReplayStoreUnavailable)
    );
    store.close().await;
}

#[tokio::test]
async fn non_positive_stored_profile_is_corrupt_not_drift() {
    for insert in [
        "INSERT INTO rullst_webhook_replay_config VALUES (1, 0, 60)",
        "INSERT INTO rullst_webhook_replay_config VALUES (1, 2, 0)",
    ] {
        let store = SqlWebhookReplayStore::connect("sqlite::memory:", 2, Duration::from_secs(60))
            .await
            .expect("SQLite replay store");
        // A table created without the CHECK constraints, as an older or
        // hand-written migration might leave it.
        rullst_orm::sqlx::query(
            "CREATE TABLE rullst_webhook_replay_config (singleton INTEGER PRIMARY KEY, max_entries INTEGER NOT NULL, ttl_seconds INTEGER NOT NULL)",
        )
        .execute(store.pool())
        .await
        .expect("unchecked config table");
        rullst_orm::sqlx::query(insert)
            .execute(store.pool())
            .await
            .expect("corrupt profile row");
        assert_eq!(
            store.prepare_schema().await,
            Err(CapitalError::WebhookReplayCorruptState),
            "{insert}"
        );
        store.close().await;
    }
}
