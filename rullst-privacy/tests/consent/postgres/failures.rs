use super::*;

pub(super) async fn schema_and_durability(url: &str, database: &mut PgConnection) {
    reset(database).await;
    let store = PostgresConsentStore::initialize(url, 10).await.unwrap();
    let gate = ConsentGate::new(store.clone()).unwrap();
    grant(&gate, 0, 1000, 1100).await.unwrap();
    sqlx::query("UPDATE rullst_consent.metadata SET capacity = 11")
        .execute(&mut *database)
        .await
        .unwrap();
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1000))
            .await,
        Err(ConsentError::StoreConfiguration)
    );
    sqlx::query("UPDATE rullst_consent.metadata SET capacity = 10")
        .execute(&mut *database)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE rullst_consent.choices SET UNLOGGED")
        .execute(&mut *database)
        .await
        .unwrap();
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1000))
            .await,
        Err(ConsentError::StoreConfiguration)
    );
    assert!(matches!(
        PostgresConsentStore::connect(url, 10).await,
        Err(ConsentError::StoreConfiguration)
    ));
    sqlx::query("ALTER TABLE rullst_consent.choices SET LOGGED")
        .execute(&mut *database)
        .await
        .unwrap();
    sqlx::query("UPDATE rullst_consent.choices SET policy_version = 'invalid/value'")
        .execute(&mut *database)
        .await
        .unwrap();
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1000))
            .await,
        Err(ConsentError::StoreConfiguration)
    );
    sqlx::query("UPDATE rullst_consent.choices SET policy_version = 'notice-v1'")
        .execute(&mut *database)
        .await
        .unwrap();
    for (disable, enable) in [
        (
            "ALTER SYSTEM SET fsync = 'off'",
            "ALTER SYSTEM SET fsync = 'on'",
        ),
        (
            "ALTER SYSTEM SET full_page_writes = 'off'",
            "ALTER SYSTEM SET full_page_writes = 'on'",
        ),
    ] {
        sqlx::query(disable).execute(&mut *database).await.unwrap();
        sqlx::query("SELECT pg_reload_conf()")
            .execute(&mut *database)
            .await
            .unwrap();
        let mut disabled = false;
        for _ in 0..100 {
            if gate
                .allows_with_clock(&subject(), &purpose(), &Clock::at(1000))
                .await
                == Err(ConsentError::StoreConfiguration)
            {
                disabled = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(disabled, "durability change must prevent processing");
        sqlx::query(enable).execute(&mut *database).await.unwrap();
        sqlx::query("SELECT pg_reload_conf()")
            .execute(&mut *database)
            .await
            .unwrap();
        let mut restored = false;
        for _ in 0..100 {
            if gate
                .allows_with_clock(&subject(), &purpose(), &Clock::at(1000))
                .await
                == Ok(true)
            {
                restored = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(restored);
    }
    sqlx::query("DROP TABLE rullst_consent.choices")
        .execute(&mut *database)
        .await
        .unwrap();
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1000))
            .await,
        Err(ConsentError::StoreConfiguration)
    );
    assert!(matches!(
        PostgresConsentStore::initialize(url, 10).await,
        Err(ConsentError::StoreConfiguration)
    ));
    store.close().await;
}

async fn blocked_query(database: &mut PgConnection) {
    for _ in 0..100 {
        let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name = 'rullst-consent-v1' AND wait_event_type = 'Lock')")
            .fetch_one(&mut *database).await.unwrap();
        if waiting {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("consent operation did not reach its expected database lock");
}

pub(super) async fn failure_cancellation_and_clock_wait(url: &str, database: &mut PgConnection) {
    reset(database).await;
    let store = PostgresConsentStore::initialize(url, 10).await.unwrap();
    let gate = ConsentGate::new(store.clone()).unwrap();
    for statement in [
        "CREATE FUNCTION rullst_consent.fail_write() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'owned fault fixture'; END $$",
        "CREATE TRIGGER fail_write BEFORE INSERT OR UPDATE ON rullst_consent.choices FOR EACH ROW EXECUTE FUNCTION rullst_consent.fail_write()",
    ] {
        sqlx::query(statement)
            .execute(&mut *database)
            .await
            .unwrap();
    }
    assert_eq!(
        grant(&gate, 0, 1000, 1100).await,
        Err(ConsentError::StoreUnavailable)
    );
    let (count, now): (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM rullst_consent.choices), last_now FROM rullst_consent.metadata")
        .fetch_one(&mut *database).await.unwrap();
    assert_eq!((count, now), (0, 0));
    sqlx::query("DROP TRIGGER fail_write ON rullst_consent.choices")
        .execute(&mut *database)
        .await
        .unwrap();

    let mut blocker = PgConnection::connect(url).await.unwrap();
    let mut tx = blocker.begin().await.unwrap();
    sqlx::query("SELECT id FROM rullst_consent.metadata FOR UPDATE")
        .execute(&mut *tx)
        .await
        .unwrap();
    let waiting = ConsentGate::new(store.clone()).unwrap();
    let task = tokio::spawn(async move { grant(&waiting, 0, 1000, 1100).await });
    blocked_query(database).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tx.rollback().await.unwrap();
    // Waiting for a new authoritative operation also observes rollback completion.
    assert_eq!(
        gate.current_with_clock(&subject(), &purpose(), &Clock::at(1000))
            .await
            .unwrap()
            .revision(),
        0
    );

    let clock = Arc::new(Clock::at(1000));
    let mut tx = blocker.begin().await.unwrap();
    sqlx::query("SELECT id FROM rullst_consent.metadata FOR UPDATE")
        .execute(&mut *tx)
        .await
        .unwrap();
    let waiting = ConsentGate::new(store.clone()).unwrap();
    let task_clock = clock.clone();
    let task = tokio::spawn(async move {
        waiting
            .choose_with_clock(
                &subject(),
                &purpose(),
                &submission(0, ConsentChoice::Granted),
                1100,
                &*task_clock,
            )
            .await
    });
    blocked_query(database).await;
    clock.set(1200);
    tx.commit().await.unwrap();
    assert_eq!(task.await.unwrap(), Err(ConsentError::InvalidConfiguration));
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1050))
            .await,
        Err(ConsentError::ClockRollback)
    );
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &Clock::at(1200))
            .await
            .unwrap()
    );
    blocker.close().await.unwrap();
    store.close().await;
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1200))
            .await,
        Err(ConsentError::StoreUnavailable)
    );
}
