#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rullst_orm::{Error, Orm, SearchEngine, set_search_engine};
use serde_json::Value;

#[derive(Debug, Clone, rullst_orm::FromRow, rullst_orm::Orm)]
#[orm(table = "scout_post_commit_models", searchable)]
struct ScoutPostCommitModel {
    pub id: i32,
    pub name: String,
}

struct VerifyingSearchEngine {
    updates: Arc<AtomicUsize>,
    deletes: Arc<AtomicUsize>,
    fail_updates: Arc<AtomicBool>,
}

#[rullst_orm::async_trait]
impl SearchEngine for VerifyingSearchEngine {
    async fn update(&self, table: &str, id: i32, payload: Value) -> Result<(), Error> {
        let query = format!("SELECT name FROM {table} WHERE id = ?");
        let persisted: (String,) = sqlx::query_as(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .fetch_one(Orm::pool()?)
            .await?;
        if payload.get("name").and_then(Value::as_str) != Some(persisted.0.as_str()) {
            return Err(Error::Internal(
                "Scout projection did not observe committed model state".to_string(),
            ));
        }
        self.updates.fetch_add(1, Ordering::SeqCst);
        if self.fail_updates.load(Ordering::SeqCst) {
            return Err(Error::Internal("simulated Scout outage".to_string()));
        }
        Ok(())
    }

    async fn delete(&self, table: &str, id: i32) -> Result<(), Error> {
        let query = format!("SELECT COUNT(*) FROM {table} WHERE id = ?");
        let persisted: (i64,) = sqlx::query_as(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .fetch_one(Orm::pool()?)
            .await?;
        if persisted.0 != 0 {
            return Err(Error::Internal(
                "Scout delete ran before the model deletion committed".to_string(),
            ));
        }
        self.deletes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn search(&self, _table: &str, _query: &str) -> Result<Vec<i32>, Error> {
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn generated_scout_projection_uses_the_managed_commit_boundary() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-scout-post-commit-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    let database_url = format!("sqlite:{}?mode=rwc", database_path.to_string_lossy());
    Orm::init(&database_url)
        .await
        .expect("initialize isolated SQLite ORM");
    sqlx::query(
        "CREATE TABLE scout_post_commit_models (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
    )
    .execute(Orm::pool().expect("ORM should be initialized"))
    .await
    .expect("create Scout fixture table");

    let updates = Arc::new(AtomicUsize::new(0));
    let deletes = Arc::new(AtomicUsize::new(0));
    let fail_updates = Arc::new(AtomicBool::new(false));
    set_search_engine(Box::new(VerifyingSearchEngine {
        updates: updates.clone(),
        deletes: deletes.clone(),
        fail_updates: fail_updates.clone(),
    }));

    let mut model = ScoutPostCommitModel {
        id: 0,
        name: "created".to_string(),
    };
    model.save().await.expect("save and project created model");
    assert_eq!(updates.load(Ordering::SeqCst), 1);

    let rollback_updates = updates.clone();
    let mut rolled_back_model = model.clone();
    let rollback = Orm::transaction(|_| {
        Box::pin(async move {
            rolled_back_model.name = "rolled back".to_string();
            rolled_back_model.save().await?;
            assert_eq!(rollback_updates.load(Ordering::SeqCst), 1);
            Err::<(), Error>(Error::Validation("force Scout rollback".to_string()))
        })
    })
    .await;
    assert!(rollback.is_err());
    assert_eq!(updates.load(Ordering::SeqCst), 1);

    model.name = "committed update".to_string();
    model.save().await.expect("project committed model update");
    assert_eq!(updates.load(Ordering::SeqCst), 2);

    fail_updates.store(true, Ordering::SeqCst);
    model.name = "durable despite projection failure".to_string();
    let projection_error = model
        .save()
        .await
        .expect_err("Scout failure after commit must remain visible");
    assert!(matches!(projection_error, Error::PostCommit(_)));
    let persisted: (String,) =
        sqlx::query_as("SELECT name FROM scout_post_commit_models WHERE id = ?")
            .bind(model.id)
            .fetch_one(Orm::pool().expect("ORM should be initialized"))
            .await
            .expect("read model after projection failure");
    assert_eq!(persisted.0, "durable despite projection failure");

    fail_updates.store(false, Ordering::SeqCst);
    model.delete().await.expect("delete and project model");
    assert_eq!(deletes.load(Ordering::SeqCst), 1);

    let _ = std::fs::remove_file(database_path);
}
