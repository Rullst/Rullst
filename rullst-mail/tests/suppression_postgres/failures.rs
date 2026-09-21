use super::support::*;
use std::time::Duration;

pub async fn run(url: &str) {
    let namespace = unique();
    let limits = PostgresSuppressionConfig::new(&namespace, 1, 2).unwrap();
    let first = PostgresSuppressionStore::initialize(url, key(), limits.clone())
        .await
        .unwrap();
    let second = PostgresSuppressionStore::connect(url, key(), limits)
        .await
        .unwrap();
    let time = now() - 60;
    let (a, b) = tokio::join!(
        first.record(event("a", "a@example.com", SuppressionReason::Manual, time)),
        second.record(event("b", "b@example.com", SuppressionReason::Manual, time))
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let (recipient, rejected) = if a.is_ok() {
        ("a@example.com", b)
    } else {
        ("b@example.com", a)
    };
    assert_eq!(rejected.unwrap_err(), SuppressionError::CapacityExceeded);
    first
        .record(event(
            "second",
            recipient,
            SuppressionReason::SpamComplaint,
            time,
        ))
        .await
        .unwrap();
    assert_eq!(
        first
            .record(event("excess", recipient, SuppressionReason::Manual, time))
            .await
            .unwrap_err(),
        SuppressionError::CapacityExceeded
    );
    assert_eq!(first.prune_events_before(time + 1).await.unwrap(), 2);
    assert_eq!(second.snapshot().await.unwrap().recipients(), 1);
    first
        .record(event("new", recipient, SuppressionReason::Manual, time))
        .await
        .unwrap();
    assert_eq!(
        second.lookup(recipient).await.unwrap().unwrap().reason(),
        SuppressionReason::SpamComplaint
    );
    first.close().await;
    second.close().await;

    let namespace = unique();
    let store = PostgresSuppressionStore::initialize(url, key(), config(&namespace))
        .await
        .unwrap();
    let raw = sqlx::PgPool::connect(url).await.unwrap();
    // Failure in the second table cannot leave a recipient or replay half-write.
    sqlx::query("ALTER TABLE rullst_mail_pg_suppression_events ADD CONSTRAINT reject_fixture_event CHECK(observed_at = 0) NOT VALID")
        .execute(&raw).await.unwrap();
    assert!(matches!(
        store
            .record(event(
                "rollback",
                "rollback@example.com",
                SuppressionReason::Manual,
                time
            ))
            .await,
        Err(SuppressionError::StorageUnavailable(_))
    ));
    assert!(
        store
            .lookup("rollback@example.com")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(store.snapshot().await.unwrap().events(), 0);
    sqlx::query(
        "ALTER TABLE rullst_mail_pg_suppression_events DROP CONSTRAINT reject_fixture_event",
    )
    .execute(&raw)
    .await
    .unwrap();
    let mut lock = raw.begin().await.unwrap();
    sqlx::query(
        "SELECT namespace FROM rullst_mail_pg_suppression_control WHERE namespace = $1 FOR UPDATE",
    )
    .bind(&namespace)
    .fetch_one(&mut *lock)
    .await
    .unwrap();
    let pending = {
        let store = store.clone();
        tokio::spawn(async move {
            store
                .record(event(
                    "cancelled",
                    "cancelled@example.com",
                    SuppressionReason::Manual,
                    time,
                ))
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!pending.is_finished());
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    lock.commit().await.unwrap();
    assert!(
        store
            .lookup("cancelled@example.com")
            .await
            .unwrap()
            .is_none()
    );
    // A real lock deadline must fail closed instead of ignoring current state.
    let mut lock = raw.begin().await.unwrap();
    sqlx::query(
        "SELECT namespace FROM rullst_mail_pg_suppression_control WHERE namespace = $1 FOR UPDATE",
    )
    .bind(&namespace)
    .fetch_one(&mut *lock)
    .await
    .unwrap();
    let started = std::time::Instant::now();
    assert!(matches!(
        store.lookup("unknown@example.com").await,
        Err(SuppressionError::StorageUnavailable(_))
    ));
    assert!(started.elapsed() < Duration::from_secs(11));
    lock.rollback().await.unwrap();
    store
        .record(event(
            "after-lock",
            "after@example.com",
            SuppressionReason::Manual,
            time,
        ))
        .await
        .unwrap();
    sqlx::query("UPDATE rullst_mail_pg_suppression_control SET last_now = $1 WHERE namespace = $2")
        .bind((now() + 3600) as i64)
        .bind(&namespace)
        .execute(&raw)
        .await
        .unwrap();
    assert!(matches!(
        store.lookup("unknown@example.com").await,
        Err(SuppressionError::InvalidConfiguration("server clock"))
    ));
    sqlx::query("UPDATE rullst_mail_pg_suppression_control SET last_now = $1 WHERE namespace = $2")
        .bind(now() as i64)
        .bind(&namespace)
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE rullst_mail_pg_suppression_recipients SET UNLOGGED")
        .execute(&raw)
        .await
        .unwrap();
    assert!(matches!(
        store.lookup("unknown@example.com").await,
        Err(SuppressionError::InvalidConfiguration("durable database"))
    ));
    assert!(
        PostgresSuppressionStore::connect(url, key(), config(&namespace))
            .await
            .is_err()
    );
    sqlx::query("ALTER TABLE rullst_mail_pg_suppression_recipients SET LOGGED")
        .execute(&raw)
        .await
        .unwrap();
    for (statement, expected) in [
        ("ALTER SYSTEM SET full_page_writes = off", "off"),
        ("ALTER SYSTEM RESET full_page_writes", "on"),
    ] {
        sqlx::query(statement).execute(&raw).await.unwrap();
        sqlx::query("SELECT pg_reload_conf()")
            .execute(&raw)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let value: String =
                    sqlx::query_scalar("SELECT current_setting('full_page_writes')")
                        .fetch_one(&raw)
                        .await
                        .unwrap();
                if value == expected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            store.lookup("unknown@example.com").await.is_ok(),
            expected == "on"
        );
    }
    store.close().await;
    role(url, &raw, &namespace).await;
    raw.close().await;
}

async fn role(url: &str, raw: &sqlx::PgPool, namespace: &str) {
    sqlx::query("CREATE ROLE rullst_mail_runtime LOGIN")
        .execute(raw)
        .await
        .unwrap();
    sqlx::query("GRANT USAGE ON SCHEMA public TO rullst_mail_runtime")
        .execute(raw)
        .await
        .unwrap();
    sqlx::query("GRANT SELECT,UPDATE ON rullst_mail_pg_suppression_control TO rullst_mail_runtime")
        .execute(raw)
        .await
        .unwrap();
    sqlx::query("GRANT SELECT,INSERT,UPDATE ON rullst_mail_pg_suppression_recipients TO rullst_mail_runtime").execute(raw).await.unwrap();
    sqlx::query(
        "GRANT SELECT,INSERT,DELETE ON rullst_mail_pg_suppression_events TO rullst_mail_runtime",
    )
    .execute(raw)
    .await
    .unwrap();
    let mut runtime = url::Url::parse(url).unwrap();
    runtime.set_username("rullst_mail_runtime").unwrap();
    let store = PostgresSuppressionStore::connect(runtime.as_str(), key(), config(namespace))
        .await
        .unwrap();
    store
        .record(event(
            "role",
            "role@example.com",
            SuppressionReason::HardBounce,
            now() - 60,
        ))
        .await
        .unwrap();
    assert!(store.lookup("role@example.com").await.unwrap().is_some());
    assert!(store.snapshot().await.unwrap().recipients() > 0);
    store.prune_events_before(now()).await.unwrap();
    assert!(store.lookup("role@example.com").await.unwrap().is_some());
    let pool = sqlx::PgPool::connect(runtime.as_str()).await.unwrap();
    assert!(
        sqlx::query("ALTER TABLE rullst_mail_pg_suppression_recipients ADD COLUMN forbidden TEXT")
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM rullst_mail_pg_suppression_recipients")
            .execute(&pool)
            .await
            .is_err()
    );
    pool.close().await;
    store.close().await;
    sqlx::query("DROP OWNED BY rullst_mail_runtime")
        .execute(raw)
        .await
        .unwrap();
    sqlx::query("DROP ROLE rullst_mail_runtime")
        .execute(raw)
        .await
        .unwrap();
}
