#![allow(dead_code)]

use rullst_capital::{
    BillingSubject, QuotaError, QuotaRequest, QuotaStore as _, SqlQuotaBackend, SqlQuotaStore,
};
#[cfg(feature = "webhook-sql")]
use rullst_capital::{CapitalError, SqlWebhookBackend, SqlWebhookReplayStore};
#[cfg(feature = "webhook-sql")]
use std::time::Duration;

pub fn handle_container_start_error(provider: &str, error: impl std::fmt::Display) {
    if std::env::var_os("RULLST_REQUIRE_TESTCONTAINERS").is_some() {
        panic!("{provider} testcontainer is required but unavailable: {error}");
    }
    eprintln!("skipping {provider} quota contract because Docker is unavailable: {error}");
}

pub async fn exercise_sql_quota(database_url: &str, backend: SqlQuotaBackend) {
    let store = SqlQuotaStore::connect(database_url)
        .await
        .expect("live SQL quota store");
    assert_eq!(store.backend(), backend);
    store.prepare_schema().await.expect("live quota schema");
    let workspace = BillingSubject::try_new("workspace", "live-team").expect("subject");
    let request = QuotaRequest::try_new(workspace.clone(), "projects", "project-live-1", 2, 3)
        .expect("request");
    let grant = store.reserve(&request).await.expect("first reservation");
    assert!(!grant.is_replay());
    assert_eq!(grant.used_after(), 2);
    assert!(store.reserve(&request).await.expect("replay").is_replay());
    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 2);

    let conflicting = QuotaRequest::try_new(workspace.clone(), "projects", "project-live-1", 1, 3)
        .expect("conflicting request");
    assert_eq!(
        store.reserve(&conflicting).await,
        Err(QuotaError::IdempotencyConflict)
    );
    let over_limit = QuotaRequest::try_new(workspace.clone(), "projects", "project-live-2", 2, 3)
        .expect("over-limit request");
    assert!(matches!(
        store.reserve(&over_limit).await,
        Err(QuotaError::LimitExceeded { .. })
    ));
    assert!(store.release(&grant).await.expect("exact release"));
    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 0);

    exercise_concurrency(&store).await;
}

#[cfg(feature = "webhook-sql")]
pub async fn exercise_sql_webhook_replay(database_url: &str, backend: SqlWebhookBackend) {
    let first = SqlWebhookReplayStore::connect(database_url, 32, Duration::from_secs(60))
        .await
        .expect("live SQL webhook replay store");
    assert_eq!(first.backend(), backend);
    first
        .prepare_schema()
        .await
        .expect("live webhook replay schema");

    let second = SqlWebhookReplayStore::connect(database_url, 32, Duration::from_secs(60))
        .await
        .expect("second live SQL webhook replay store");
    second
        .prepare_schema()
        .await
        .expect("shared webhook replay schema");
    first
        .check_and_record_event_key("stripe", "evt_live_restart_1")
        .await
        .expect("first durable webhook claim");
    assert!(matches!(
        second
            .check_and_record_event_key("stripe", "evt_live_restart_1")
            .await,
        Err(CapitalError::WebhookReplay(_))
    ));

    let first = std::sync::Arc::new(first);
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let store = first.clone();
        tasks.spawn(async move {
            store
                .check_and_record_payload("stripe", b"live-concurrent-event")
                .await
        });
    }
    let mut accepted = 0;
    let mut replayed = 0;
    while let Some(result) = tasks.join_next().await {
        match result.expect("live webhook replay task") {
            Ok(()) => accepted += 1,
            Err(CapitalError::WebhookReplay(_)) => replayed += 1,
            Err(error) => panic!("unexpected live webhook replay error: {error}"),
        }
    }
    assert_eq!(accepted, 1);
    assert_eq!(replayed, 7);

    let (insert_effect_sql, count_effect_sql) = if backend == SqlWebhookBackend::Postgres {
        (
            "INSERT INTO rullst_webhook_replay_effects (event_name) VALUES ($1)",
            "SELECT COUNT(*) FROM rullst_webhook_replay_effects WHERE event_name = $1",
        )
    } else {
        (
            "INSERT INTO rullst_webhook_replay_effects (event_name) VALUES (?)",
            "SELECT COUNT(*) FROM rullst_webhook_replay_effects WHERE event_name = ?",
        )
    };
    rullst_orm::sqlx::query(
        "CREATE TABLE IF NOT EXISTS rullst_webhook_replay_effects (event_name VARCHAR(128) PRIMARY KEY NOT NULL)",
    )
    .execute(first.pool())
    .await
    .expect("live webhook domain fixture schema");

    let mut rolled_back = first
        .pool()
        .begin()
        .await
        .expect("live webhook rollback transaction");
    first
        .check_and_record_event_key_with_transaction(
            &mut rolled_back,
            "stripe",
            "evt_live_transaction_1",
        )
        .await
        .expect("live transactional webhook claim");
    rullst_orm::sqlx::query(insert_effect_sql)
        .bind("rolled-back")
        .execute(&mut *rolled_back)
        .await
        .expect("live transactional domain effect");
    rolled_back.rollback().await.expect("live webhook rollback");

    let mut committed = second
        .pool()
        .begin()
        .await
        .expect("live webhook commit transaction");
    second
        .check_and_record_event_key_with_transaction(
            &mut committed,
            "stripe",
            "evt_live_transaction_1",
        )
        .await
        .expect("rolled-back claim can be retried");
    rullst_orm::sqlx::query(insert_effect_sql)
        .bind("committed")
        .execute(&mut *committed)
        .await
        .expect("live committed domain effect");
    committed.commit().await.expect("live webhook commit");
    assert!(matches!(
        first
            .check_and_record_event_key("stripe", "evt_live_transaction_1")
            .await,
        Err(CapitalError::WebhookReplay(_))
    ));
    let committed_effects = rullst_orm::sqlx::query_scalar::<_, i64>(count_effect_sql)
        .bind("committed")
        .fetch_one(first.pool())
        .await
        .expect("live committed domain effect count");
    assert_eq!(committed_effects, 1);

    let drifted = SqlWebhookReplayStore::connect(database_url, 33, Duration::from_secs(60))
        .await
        .expect("alternate webhook replay profile");
    assert_eq!(
        drifted.prepare_schema().await,
        Err(CapitalError::WebhookReplayConfigurationDrift)
    );

    drifted.close().await;
    second.close().await;
    first.close().await;
}

async fn exercise_concurrency(store: &SqlQuotaStore) {
    let workspace = BillingSubject::try_new("workspace", "live-concurrency").expect("subject");
    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..12 {
        let task_store = store.clone();
        let task_subject = workspace.clone();
        tasks.spawn(async move {
            let request =
                QuotaRequest::try_new(task_subject, "seats", format!("member-{index}"), 1, 4)?;
            task_store.reserve(&request).await
        });
    }
    let mut granted = 0;
    let mut denied = 0;
    while let Some(result) = tasks.join_next().await {
        match result.expect("live quota task") {
            Ok(_) => granted += 1,
            Err(QuotaError::LimitExceeded { .. }) => denied += 1,
            Err(error) => panic!("unexpected live quota error: {error}"),
        }
    }
    assert_eq!(granted, 4);
    assert_eq!(denied, 8);
    assert_eq!(store.usage(&workspace, "seats").await.unwrap(), 4);
}
