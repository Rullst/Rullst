use super::*;

pub(super) async fn schema_and_metadata_drift(url: &str, database: &mut PgConnection) {
    reset(database).await;
    let store = PostgresReplayStore::initialize(url, 1).await.unwrap();
    assert!(store.claim(nonce(), 1100, 1000).await.unwrap());
    assert!(matches!(
        PostgresReplayStore::connect(url, 2).await,
        Err(AgeError::StoreConfiguration)
    ));
    assert!(matches!(
        PostgresReplayStore::initialize(url, 2).await,
        Err(AgeError::StoreConfiguration)
    ));
    sqlx::query("UPDATE rullst_age_replay.metadata SET schema_version = 99")
        .execute(&mut *database)
        .await
        .unwrap();
    assert_eq!(
        store.claim(nonce(), 1100, 1000).await,
        Err(AgeError::StoreConfiguration)
    );
    sqlx::query("DELETE FROM rullst_age_replay.metadata")
        .execute(&mut *database)
        .await
        .unwrap();
    assert_eq!(
        store.claim(nonce(), 1100, 1000).await,
        Err(AgeError::StoreConfiguration)
    );
    assert!(matches!(
        PostgresReplayStore::initialize(url, 1).await,
        Err(AgeError::StoreConfiguration)
    ));
    sqlx::query("DROP TABLE rullst_age_replay.claims")
        .execute(&mut *database)
        .await
        .unwrap();
    assert!(matches!(
        PostgresReplayStore::initialize(url, 1).await,
        Err(AgeError::StoreConfiguration)
    ));
    store.close().await;
    reset(database).await;
    let store = PostgresReplayStore::initialize(url, 1).await.unwrap();
    sqlx::query("ALTER TABLE rullst_age_replay.claims SET UNLOGGED")
        .execute(&mut *database)
        .await
        .unwrap();
    assert_eq!(
        store.claim(nonce(), 1100, 1000).await,
        Err(AgeError::StoreConfiguration)
    );
    assert!(matches!(
        PostgresReplayStore::connect(url, 1).await,
        Err(AgeError::StoreConfiguration)
    ));
    store.close().await;
}

pub(super) async fn atomic_failure_and_cancellation(url: &str, database: &mut PgConnection) {
    reset(database).await;
    let store = PostgresReplayStore::initialize(url, 10).await.unwrap();
    let verifier = AgeVerifier::new(issuer(), store.clone()).unwrap();
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    for statement in [
        "CREATE FUNCTION rullst_age_replay.fail_claim() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected failure'; END; $$",
        "CREATE TRIGGER fail_claim BEFORE INSERT ON rullst_age_replay.claims FOR EACH ROW EXECUTE FUNCTION rullst_age_replay.fail_claim()",
    ] {
        sqlx::query(statement)
            .execute(&mut *database)
            .await
            .unwrap();
    }
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001)).await,
        Err(AgeError::StoreUnavailable)
    );
    let last: i64 = sqlx::query_scalar("SELECT last_now FROM rullst_age_replay.metadata")
        .fetch_one(&mut *database)
        .await
        .unwrap();
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_age_replay.claims")
        .fetch_one(&mut *database)
        .await
        .unwrap();
    assert_eq!((last, rows), (0, 0));
    sqlx::query("DROP TRIGGER fail_claim ON rullst_age_replay.claims")
        .execute(&mut *database)
        .await
        .unwrap();
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001))
            .await
            .unwrap()
            .decision(),
        AgeDecision::Allowed
    );

    sqlx::query("BEGIN").execute(&mut *database).await.unwrap();
    sqlx::query("SELECT id FROM rullst_age_replay.metadata FOR UPDATE")
        .execute(&mut *database)
        .await
        .unwrap();
    let cancelled = nonce();
    assert!(
        tokio::time::timeout(
            Duration::from_millis(150),
            store.claim(cancelled, 1100, 1001)
        )
        .await
        .is_err()
    );
    sqlx::query("ROLLBACK")
        .execute(&mut *database)
        .await
        .unwrap();
    // Cancellation can consume a claim but never grant access. Either database
    // outcome must release its lock and converge to rejection after one claim.
    tokio::time::timeout(Duration::from_secs(10), store.claim(cancelled, 1100, 1001))
        .await
        .unwrap()
        .unwrap();
    assert!(!store.claim(cancelled, 1100, 1001).await.unwrap());
    store.close().await;
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001)).await,
        Err(AgeError::StoreUnavailable)
    );

    let killed = PostgresReplayStore::connect(url, 10).await.unwrap();
    sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE application_name = 'rullst-age-replay-v1'")
        .execute(&mut *database).await.unwrap();
    // A fresh connection can recover availability; it cannot forget consumption.
    let verifier = AgeVerifier::new(issuer(), killed.clone()).unwrap();
    let result = assess(&verifier, &challenge, &FixedClock(1001)).await;
    assert!(matches!(
        result,
        Err(AgeError::Replay | AgeError::StoreUnavailable)
    ));
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001)).await,
        Err(AgeError::Replay)
    );
    killed.close().await;
}

pub(super) async fn durability_settings(url: &str, database: &mut PgConnection) {
    reset(database).await;
    // Application/database defaults must not disable synchronous commit in the
    // store's own sessions. This is an owned disposable database only.
    sqlx::query("ALTER DATABASE rullst_privacy_contract SET synchronous_commit = off")
        .execute(&mut *database)
        .await
        .unwrap();
    let store = PostgresReplayStore::initialize(url, 10).await.unwrap();
    assert!(store.claim(nonce(), 1100, 1000).await.unwrap());
    sqlx::query("ALTER DATABASE rullst_privacy_contract RESET synchronous_commit")
        .execute(&mut *database)
        .await
        .unwrap();
    sqlx::query("ALTER SYSTEM SET fsync = off")
        .execute(&mut *database)
        .await
        .unwrap();
    sqlx::query("SELECT pg_reload_conf()")
        .execute(&mut *database)
        .await
        .unwrap();
    for _ in 0..30 {
        let disabled: bool = sqlx::query_scalar("SELECT current_setting('fsync') = 'off'")
            .fetch_one(&mut *database)
            .await
            .unwrap();
        if disabled {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(matches!(
        PostgresReplayStore::connect(url, 10).await,
        Err(AgeError::StoreConfiguration)
    ));
    assert_eq!(
        store.claim(nonce(), 1100, 1000).await,
        Err(AgeError::StoreConfiguration)
    );
    sqlx::query("ALTER SYSTEM SET fsync = on")
        .execute(&mut *database)
        .await
        .unwrap();
    sqlx::query("SELECT pg_reload_conf()")
        .execute(&mut *database)
        .await
        .unwrap();
    for _ in 0..30 {
        let restored: bool = sqlx::query_scalar("SELECT current_setting('fsync') = 'on'")
            .fetch_one(&mut *database)
            .await
            .unwrap();
        if restored {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    store.close().await;
}
