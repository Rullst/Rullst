#![allow(dead_code)]

use rullst_capital::{
    BillingSubject, QuotaError, QuotaRequest, QuotaStore as _, SqlQuotaBackend, SqlQuotaStore,
};

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
