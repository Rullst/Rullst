#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rullst_orm::{Error, ModelCommittedEvent, ModelOperation, Orm, after_commit};

#[derive(Debug, Clone, rullst_orm::FromRow, rullst_orm::Orm)]
#[orm(table = "post_commit_models")]
struct PostCommitModel {
    pub id: i32,
    pub name: String,
}

struct CommitObserver {
    operations: Arc<std::sync::Mutex<Vec<ModelOperation>>>,
}

#[rullst_orm::async_trait]
impl PostCommitModelObserver for CommitObserver {
    async fn committed(&self, event: &ModelCommittedEvent) -> Result<(), Error> {
        self.operations
            .lock()
            .expect("lock test observer operations")
            .push(event.operation);
        Ok(())
    }
}

#[tokio::test]
async fn managed_transactions_run_effects_only_after_a_successful_commit() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-post-commit-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    let database_url = format!("sqlite:{}?mode=rwc", database_path.to_string_lossy());
    Orm::init(&database_url)
        .await
        .expect("initialize isolated SQLite ORM");
    sqlx::query("CREATE TABLE post_commit_records (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(Orm::pool().expect("ORM should be initialized"))
        .await
        .expect("create post-commit fixture table");
    sqlx::query("CREATE TABLE post_commit_models (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(Orm::pool().expect("ORM should be initialized"))
        .await
        .expect("create observer fixture table");

    let committed_calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = committed_calls.clone();
    let transaction_calls = committed_calls.clone();
    Orm::transaction(|_| {
        Box::pin(async move {
            let insert = sqlx::query("INSERT INTO post_commit_records (id, name) VALUES (?, ?)")
                .bind(1_i64)
                .bind("committed");
            rullst_orm::execute_query!(insert, execute, pool)?;
            after_commit(move || async move {
                let count: (i64,) =
                    sqlx::query_as("SELECT COUNT(*) FROM post_commit_records WHERE id = ?")
                        .bind(1_i64)
                        .fetch_one(Orm::pool()?)
                        .await?;
                if count.0 != 1 {
                    return Err(Error::Internal(
                        "post-commit callback could not observe the committed row".to_string(),
                    ));
                }
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await?;
            assert_eq!(transaction_calls.load(Ordering::SeqCst), 0);
            Ok::<(), Error>(())
        })
    })
    .await
    .expect("commit managed transaction");
    assert_eq!(committed_calls.load(Ordering::SeqCst), 1);

    let rolled_back_calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = rolled_back_calls.clone();
    let rollback = Orm::transaction(|_| {
        Box::pin(async move {
            let insert = sqlx::query("INSERT INTO post_commit_records (id, name) VALUES (?, ?)")
                .bind(2_i64)
                .bind("rolled back");
            rullst_orm::execute_query!(insert, execute, pool)?;
            after_commit(move || async move {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await?;
            Err::<(), Error>(Error::Validation("force rollback".to_string()))
        })
    })
    .await;
    assert!(rollback.is_err());
    assert_eq!(rolled_back_calls.load(Ordering::SeqCst), 0);

    let continued_calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = continued_calls.clone();
    let post_commit_failure = Orm::transaction(|_| {
        Box::pin(async move {
            let insert = sqlx::query("INSERT INTO post_commit_records (id, name) VALUES (?, ?)")
                .bind(3_i64)
                .bind("committed before callback error");
            rullst_orm::execute_query!(insert, execute, pool)?;
            after_commit(|| async { Err(Error::CacheError("simulated cache outage".to_string())) })
                .await?;
            after_commit(move || async move {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await?;
            Ok::<(), Error>(())
        })
    })
    .await
    .expect_err("a failed post-commit callback must remain visible");
    assert!(matches!(post_commit_failure, Error::PostCommit(_)));
    assert_eq!(continued_calls.load(Ordering::SeqCst), 1);

    let persisted: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM post_commit_records")
        .fetch_one(Orm::pool().expect("ORM should be initialized"))
        .await
        .expect("count durable rows");
    assert_eq!(persisted.0, 2);

    let removed_handle_calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = removed_handle_calls.clone();
    let removed_handle = Orm::transaction(|transaction| {
        Box::pin(async move {
            after_commit(move || async move {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await?;
            let transaction = transaction.lock().await.take().ok_or_else(|| {
                Error::Internal("test transaction handle was already absent".to_string())
            })?;
            transaction.rollback().await?;
            Ok::<(), Error>(())
        })
    })
    .await;
    assert!(matches!(removed_handle, Err(Error::Internal(_))));
    assert_eq!(removed_handle_calls.load(Ordering::SeqCst), 0);

    let operations = Arc::new(std::sync::Mutex::new(Vec::new()));
    PostCommitModel::observe(Arc::new(CommitObserver {
        operations: operations.clone(),
    }));

    let rollback_operations = operations.clone();
    let model_rollback = Orm::transaction(|_| {
        Box::pin(async move {
            let mut model = PostCommitModel {
                id: 0,
                name: "observer rollback".to_string(),
            };
            model.save().await?;
            assert!(
                rollback_operations
                    .lock()
                    .expect("lock rollback operations")
                    .is_empty()
            );
            Err::<(), Error>(Error::Validation("force model rollback".to_string()))
        })
    })
    .await;
    assert!(model_rollback.is_err());
    assert!(
        operations
            .lock()
            .expect("lock operations after rollback")
            .is_empty()
    );

    let commit_operations = operations.clone();
    Orm::transaction(|_| {
        Box::pin(async move {
            let mut model = PostCommitModel {
                id: 0,
                name: "observer commit".to_string(),
            };
            model.save().await?;
            assert!(
                commit_operations
                    .lock()
                    .expect("lock operations before commit")
                    .is_empty()
            );
            Ok::<(), Error>(())
        })
    })
    .await
    .expect("commit model transaction");
    assert_eq!(
        operations
            .lock()
            .expect("lock operations after commit")
            .as_slice(),
        &[ModelOperation::Created]
    );

    let mut direct = PostCommitModel {
        id: 0,
        name: "direct commit".to_string(),
    };
    direct.save().await.expect("commit direct model save");
    assert_eq!(
        operations
            .lock()
            .expect("lock operations after direct save")
            .as_slice(),
        &[ModelOperation::Created, ModelOperation::Created]
    );

    let _ = std::fs::remove_file(database_path);
}
