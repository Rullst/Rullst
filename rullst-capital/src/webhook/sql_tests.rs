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
