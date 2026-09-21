use super::support::*;

pub async fn run(url: &str) {
    let namespace = unique();
    let clock = ManualClock::new();
    let admin = admin(url).await;
    let store = open(url, &namespace, &clock).await;
    store
        .create(definition("one", &clock, MissedRunPolicy::CatchUp))
        .await
        .unwrap();
    // Force a real INSERT failure after the namespace lock, then verify rollback.
    sqlx::query("ALTER TABLE rullst_recurring_occurrences ADD CONSTRAINT fixture_reject CHECK (false) NOT VALID")
        .execute(&admin).await.unwrap();
    assert_eq!(store.tick(1).await, Err(RecurringError::Storage));
    sqlx::query("ALTER TABLE rullst_recurring_occurrences DROP CONSTRAINT fixture_reject")
        .execute(&admin)
        .await
        .unwrap();
    assert_eq!(store.tick(1).await.unwrap().len(), 1);
    // A cancelled SQL future cannot leave a claimed occurrence behind.
    let mut tx = admin.begin().await.unwrap();
    sqlx::query("SELECT namespace FROM rullst_recurring_control WHERE namespace=$1 FOR UPDATE")
        .bind(&namespace)
        .execute(&mut *tx)
        .await
        .unwrap();
    let clone = store.clone();
    let task = tokio::spawn(async move { clone.claim(1).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tx.rollback().await.unwrap();
    let lease = store.claim(1).await.unwrap().remove(0);
    assert_eq!(lease.metadata().attempts(), 1);
    // Time is rechecked after waiting, not merely when the operation starts.
    let mut tx = admin.begin().await.unwrap();
    sqlx::query("SELECT namespace FROM rullst_recurring_control WHERE namespace=$1 FOR UPDATE")
        .bind(&namespace)
        .execute(&mut *tx)
        .await
        .unwrap();
    let clone = store.clone();
    let task = tokio::spawn(async move { clone.schedules(None, 1).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    clock.advance(-1);
    tx.rollback().await.unwrap();
    assert_eq!(task.await.unwrap(), Err(RecurringError::Clock));
    clock.advance(1);
    assert!(
        PostgresRecurringStore::connect(
            url,
            RecurringConfig::new(&namespace, 2, 3).unwrap(),
            keys(),
            clock.clone()
        )
        .await
        .is_err()
    );
    let wrong = MessagingKeyring::new(MessagingStorageKey::try_new("fixture", [9; 32]).unwrap());
    assert!(
        PostgresRecurringStore::connect(url, config(&namespace), wrong, clock.clone())
            .await
            .is_err()
    );
    // The database deadline bounds a real lock wait without cancelling it in the caller.
    let mut tx = admin.begin().await.unwrap();
    sqlx::query("SELECT namespace FROM rullst_recurring_control WHERE namespace=$1 FOR UPDATE")
        .bind(&namespace)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(7), store.schedules(None, 1))
            .await
            .unwrap(),
        Err(RecurringError::Storage)
    );
    tx.rollback().await.unwrap();
    // Capacity must preserve every due time, including earlier rows in the batch.
    let tiny_ns = unique();
    let tiny_cfg = RecurringConfig::new(&tiny_ns, 1, 1).unwrap();
    let tiny = PostgresRecurringStore::initialize(url, tiny_cfg, keys(), clock.clone())
        .await
        .unwrap();
    tiny.create(definition("one", &clock, MissedRunPolicy::CatchUp))
        .await
        .unwrap();
    clock.advance(60_000);
    assert_eq!(tiny.tick(2).await, Err(RecurringError::Capacity));
    assert!(tiny.occurrences(None, 10).await.unwrap().is_empty());
    assert_eq!(tiny.tick(1).await.unwrap().len(), 1);
    // A runtime account needs no DDL, and durability changes must fail closed.
    sqlx::query("CREATE ROLE recurring_runtime LOGIN")
        .execute(&admin)
        .await
        .unwrap();
    for statement in [
        "GRANT USAGE ON SCHEMA public TO recurring_runtime",
        "GRANT SELECT,UPDATE ON rullst_recurring_control TO recurring_runtime",
        "GRANT SELECT,INSERT,UPDATE ON rullst_recurring_definitions TO recurring_runtime",
        "GRANT SELECT,INSERT,UPDATE,DELETE ON rullst_recurring_occurrences TO recurring_runtime",
    ] {
        sqlx::query(statement).execute(&admin).await.unwrap();
    }
    let mut restricted = url::Url::parse(url).unwrap();
    restricted.set_username("recurring_runtime").unwrap();
    let runtime = PostgresRecurringStore::connect(
        restricted.as_str(),
        config(&namespace),
        keys(),
        clock.clone(),
    )
    .await
    .unwrap();
    runtime.cancel("one").await.unwrap();
    assert!(runtime.claim(1).await.unwrap().is_empty());
    assert!(
        PostgresRecurringStore::initialize(
            restricted.as_str(),
            config(&namespace),
            keys(),
            clock.clone()
        )
        .await
        .is_err()
    );
    runtime.close().await;
    // A delivery window cannot be extended by retry or an abandoned lease.
    clock.advance(86_400_001);
    let expired = tiny.occurrences(None, 1).await.unwrap().remove(0);
    assert_eq!(expired.state(), OccurrenceState::DeadLetter);
    assert!(tiny.retry_failed(expired.id()).await.is_err());
    assert!(tiny.claim(1).await.unwrap().is_empty());
    sqlx::query("ALTER TABLE rullst_recurring_occurrences SET UNLOGGED")
        .execute(&admin)
        .await
        .unwrap();
    assert_eq!(store.tick(1).await, Err(RecurringError::Configuration));
    sqlx::query("ALTER TABLE rullst_recurring_occurrences SET LOGGED")
        .execute(&admin)
        .await
        .unwrap();
    tiny.close().await;
    store.close().await;
    admin.close().await;
}
