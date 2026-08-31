use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn subject(id: &str) -> BillingSubject {
    BillingSubject::try_new("workspace", id).expect("valid workspace subject")
}

fn request(subject: &BillingSubject, event_key: &str, units: u64, limit: u64) -> QuotaRequest {
    QuotaRequest::try_new(subject.clone(), "projects", event_key, units, limit)
        .expect("valid quota request")
}

#[test]
fn subjects_and_requests_are_bounded_and_redacted() {
    let tenant = TenantContext::try_new("school-17").expect("valid tenant");
    let tenant_subject = BillingSubject::from_tenant(&tenant).expect("tenant subject");
    assert_eq!(tenant_subject.kind(), "tenant");
    assert_eq!(tenant_subject.id(), "school-17");
    assert!(!format!("{tenant_subject:?}").contains("school-17"));

    assert!(BillingSubject::try_new("", "id").is_err());
    assert!(BillingSubject::try_new("workspace", "../other").is_err());
    assert!(QuotaRequest::try_new(subject("acme"), "projects", "event", 0, 10).is_err());
    assert!(QuotaRequest::try_new(subject("acme"), "projects", "event", 1, 0).is_err());
    assert!(QuotaRequest::try_new(subject("acme"), "bad feature", "event", 1, 1).is_err());

    let request = request(&subject("secret-workspace"), "secret-event", 1, 2);
    let debug = format!("{request:?}");
    assert!(!debug.contains("secret-workspace"));
    assert!(!debug.contains("secret-event"));
}

#[tokio::test]
async fn one_workspace_shares_usage_with_exact_replay_and_release() {
    let store = InMemoryQuotaStore::default();
    let workspace = subject("acme");
    let first_request = request(&workspace, "project-1", 2, 3);
    let first = store.reserve(&first_request).await.expect("first grant");
    assert!(!first.is_replay());
    assert_eq!(first.used_after(), 2);
    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 2);

    let replay = store.reserve(&first_request).await.expect("exact replay");
    assert!(replay.is_replay());
    assert_eq!(replay.used_after(), 2);
    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 2);

    let over_limit = store.reserve(&request(&workspace, "project-2", 2, 3)).await;
    assert_eq!(
        over_limit,
        Err(QuotaError::LimitExceeded {
            used: 2,
            requested: 2,
            limit: 3,
        })
    );
    assert_eq!(
        store.reserve(&request(&workspace, "project-1", 1, 3)).await,
        Err(QuotaError::IdempotencyConflict)
    );

    let other_workspace = subject("other-workspace");
    assert!(
        store
            .reserve(&request(&other_workspace, "project-1", 3, 3))
            .await
            .is_ok()
    );
    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 2);
    assert_eq!(store.usage(&other_workspace, "projects").await.unwrap(), 3);

    let forged = QuotaGrant {
        request: first.request().clone(),
        used_after: first.used_after(),
        claim_token: "00000000000000000000000000000000".to_string(),
        replay: false,
    };
    assert_eq!(store.release(&forged).await, Err(QuotaError::GrantMismatch));
    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 2);

    assert!(store.release(&first).await.expect("exact release"));
    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 0);
    assert!(!store.release(&first).await.expect("release replay"));
}

#[tokio::test]
async fn quota_gate_skips_replays_and_compensates_operation_failure() {
    let gate = QuotaGate::new(InMemoryQuotaStore::default());
    let workspace = subject("acme-gate");
    let calls = Arc::new(AtomicUsize::new(0));
    let first_request = request(&workspace, "create-1", 1, 1);
    let first_calls = Arc::clone(&calls);
    let first = gate
        .execute(&first_request, move || async move {
            first_calls.fetch_add(1, Ordering::SeqCst);
            Ok::<_, &'static str>("created")
        })
        .await
        .expect("first execution");
    assert!(matches!(first, QuotaExecution::Executed { .. }));

    let replay_calls = Arc::clone(&calls);
    let replay = gate
        .execute(&first_request, move || async move {
            replay_calls.fetch_add(1, Ordering::SeqCst);
            Ok::<_, &'static str>("duplicate")
        })
        .await
        .expect("safe replay");
    assert!(matches!(replay, QuotaExecution::Replay(_)));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let failing = request(&workspace, "replacement", 1, 2);
    let error = gate
        .execute(&failing, || async { Err::<(), _>("domain failure") })
        .await
        .expect_err("operation fails");
    assert!(matches!(
        error,
        QuotaExecutionError::Operation("domain failure")
    ));
    assert_eq!(gate.store().usage(&workspace, "projects").await.unwrap(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_members_cannot_overrun_the_shared_limit() {
    let store = InMemoryQuotaStore::default();
    let workspace = subject("concurrent-workspace");
    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..32 {
        let task_store = store.clone();
        let task_subject = workspace.clone();
        tasks.spawn(async move {
            task_store
                .reserve(&request(
                    &task_subject,
                    &format!("member-operation-{index}"),
                    1,
                    7,
                ))
                .await
        });
    }
    let mut granted = 0;
    let mut denied = 0;
    while let Some(result) = tasks.join_next().await {
        match result.expect("quota task") {
            Ok(_) => granted += 1,
            Err(QuotaError::LimitExceeded { .. }) => denied += 1,
            Err(error) => panic!("unexpected quota error: {error}"),
        }
    }
    assert_eq!(granted, 7);
    assert_eq!(denied, 25);
    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 7);
}

#[cfg(feature = "quota-sql")]
async fn sqlite_store(label: &str) -> (SqlQuotaStore, std::path::PathBuf) {
    let token = random_claim_token().expect("test file token");
    let path = std::env::temp_dir().join(format!("rullst-quota-{label}-{token}.sqlite"));
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let store = SqlQuotaStore::connect(url)
        .await
        .expect("SQLite quota store");
    store.prepare_schema().await.expect("quota schema");
    (store, path)
}

#[cfg(feature = "quota-sql")]
async fn close_and_remove(store: SqlQuotaStore, path: &std::path::Path) {
    store.pool().close().await;
    std::fs::remove_file(path).expect("remove temporary quota database");
}

#[cfg(feature = "quota-sql")]
#[tokio::test]
async fn sqlite_store_persists_replay_conflict_and_exact_release() {
    let (store, path) = sqlite_store("lifecycle").await;
    let workspace = subject("sql-workspace");
    let first_request = request(&workspace, "sql-event-1", 2, 3);
    let grant = store.reserve(&first_request).await.expect("SQL grant");
    assert!(!grant.is_replay());
    assert_eq!(grant.used_after(), 2);

    let replay = store.reserve(&first_request).await.expect("SQL replay");
    assert!(replay.is_replay());
    assert_eq!(
        store
            .reserve(&request(&workspace, "sql-event-1", 1, 3))
            .await,
        Err(QuotaError::IdempotencyConflict)
    );
    assert!(matches!(
        store
            .reserve(&request(&workspace, "sql-event-2", 2, 3))
            .await,
        Err(QuotaError::LimitExceeded { .. })
    ));
    assert!(store.release(&grant).await.expect("SQL release"));
    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 0);

    close_and_remove(store, &path).await;
}

#[cfg(feature = "quota-sql")]
#[tokio::test]
async fn caller_transaction_rolls_quota_and_domain_write_back_together() {
    let (store, path) = sqlite_store("transaction").await;
    rullst_orm::sqlx::query(
        "CREATE TABLE projects (id TEXT PRIMARY KEY NOT NULL, workspace_id TEXT NOT NULL)",
    )
    .execute(store.pool())
    .await
    .expect("project table");
    let workspace = subject("atomic-workspace");
    let quota_request = request(&workspace, "atomic-project", 1, 1);

    let mut transaction = store.pool().begin().await.expect("begin transaction");
    store
        .reserve_with_transaction(&mut transaction, &quota_request)
        .await
        .expect("transactional reservation");
    rullst_orm::sqlx::query("INSERT INTO projects (id, workspace_id) VALUES (?, ?)")
        .bind("project-1")
        .bind(workspace.id())
        .execute(&mut *transaction)
        .await
        .expect("domain insert");
    transaction.rollback().await.expect("atomic rollback");

    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 0);
    let count = rullst_orm::sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects")
        .fetch_one(store.pool())
        .await
        .expect("project count");
    assert_eq!(count, 0);
    assert!(!store.reserve(&quota_request).await.unwrap().is_replay());

    close_and_remove(store, &path).await;
}

#[cfg(feature = "quota-sql")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_concurrent_reservations_enforce_the_limit() {
    let (store, path) = sqlite_store("concurrency").await;
    let workspace = subject("sql-concurrent-workspace");
    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..12 {
        let task_store = store.clone();
        let task_subject = workspace.clone();
        tasks.spawn(async move {
            task_store
                .reserve(&request(
                    &task_subject,
                    &format!("sql-operation-{index}"),
                    1,
                    4,
                ))
                .await
        });
    }
    let mut granted = 0;
    let mut denied = 0;
    while let Some(result) = tasks.join_next().await {
        match result.expect("SQL quota task") {
            Ok(_) => granted += 1,
            Err(QuotaError::LimitExceeded { .. }) => denied += 1,
            Err(error) => panic!("unexpected SQL quota error: {error}"),
        }
    }
    assert_eq!(granted, 4);
    assert_eq!(denied, 8);
    assert_eq!(store.usage(&workspace, "projects").await.unwrap(), 4);

    close_and_remove(store, &path).await;
}
