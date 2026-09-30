#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::{Error, Orm, Policy};

#[derive(Clone, Debug, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "mutation_callback_records")]
struct PlainRecord {
    id: i32,
    name: String,
}

#[derive(Clone, Debug, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "mutation_callback_records", policy = "QueriesDatabase")]
struct PolicyRecord {
    id: i32,
    name: String,
}

struct QueriesDatabase;

#[async_trait::async_trait]
impl Policy<PolicyRecord> for QueriesDatabase {
    async fn can_create(_: &PolicyRecord) -> Result<bool, Error> {
        Ok(PlainRecord::query().count().await? > 0)
    }

    async fn can_delete(_: &PolicyRecord) -> Result<bool, Error> {
        Ok(PlainRecord::query().count().await? > 0)
    }
}

#[derive(Clone, Debug, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "mutation_callback_records", after_save = "query_database")]
struct HookRecord {
    id: i32,
    name: String,
}

impl HookRecord {
    async fn query_database(&mut self) -> Result<(), Error> {
        Orm::raw("SELECT id FROM mutation_callback_records")
            .map_to::<(i32,)>()
            .await?;
        Ok(())
    }
}

#[derive(Clone, Debug, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "mutation_callback_records", after_save = "reject_marked_name")]
struct RejectingHookRecord {
    id: i32,
    name: String,
}

impl RejectingHookRecord {
    async fn reject_marked_name(&mut self) -> Result<(), Error> {
        if self.name.starts_with("rejected") {
            return Err(Error::Validation(
                "after_save rejected the record".to_string(),
            ));
        }
        Ok(())
    }
}

async fn named_rows(name: &str) -> i64 {
    PlainRecord::query()
        .where_eq("name", name)
        .count()
        .await
        .expect("count named rows")
}

async fn rejected_reentry<T: std::fmt::Debug>(
    future: impl std::future::Future<Output = Result<T, Error>>,
) {
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), future)
        .await
        .expect("mutation callbacks must fail before reacquiring their own transaction");
    assert!(
        matches!(&result, Err(Error::Validation(message)) if message.contains("reentrant")),
        "{result:?}"
    );
}

#[tokio::test]
async fn mutation_policy_and_hooks_reject_reentry_without_losing_atomicity() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("isolated mutation database");
    rullst_orm::_sqlx::query(
        "CREATE TABLE mutation_callback_records (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
    )
    .execute(Orm::pool().expect("pool"))
    .await
    .expect("create fixture");
    let mut seed = PlainRecord {
        id: 0,
        name: "preserved".to_string(),
    };
    seed.save().await.expect("seed record");

    let mut denied_create = PolicyRecord {
        id: 0,
        name: "not inserted".to_string(),
    };
    rejected_reentry(denied_create.save()).await;
    assert_eq!(denied_create.id, 0);
    let existing = PolicyRecord::find(seed.id)
        .await
        .expect("load protected fixture")
        .expect("exists");
    rejected_reentry(existing.delete()).await;

    let mut denied_hook = HookRecord {
        id: 0,
        name: "rolled back".to_string(),
    };
    rejected_reentry(denied_hook.save()).await;
    assert_eq!(
        denied_hook.id, 0,
        "failed after_save must restore the inserted model ID"
    );
    assert_eq!(
        PlainRecord::all()
            .await
            .expect("read after rejected writes")
            .len(),
        1
    );

    let managed = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        Orm::transaction(|_| {
            Box::pin(async {
                let mut row = PolicyRecord {
                    id: 0,
                    name: "denied in managed transaction".to_string(),
                };
                row.save().await
            })
        }),
    )
    .await
    .expect("managed policy reentry must not lock forever");
    assert!(matches!(managed, Err(Error::DatabaseError(message)) if message.contains("reentrant")));
    assert_eq!(
        PlainRecord::all()
            .await
            .expect("rollback preserves data")
            .len(),
        1
    );

    // Completing a callback scope must not poison later unrelated transactions.
    Orm::transaction(|_| {
        Box::pin(async {
            let mut row = PlainRecord {
                id: 0,
                name: "normal later write".to_string(),
            };
            row.save().await
        })
    })
    .await
    .expect("subsequent non-reentrant managed transaction");
    assert_eq!(PlainRecord::all().await.expect("final data").len(), 2);

    // A save that fails after its INSERT and is caught inside the caller's
    // transaction must not leave the row behind when that transaction commits.
    Orm::transaction(|_| {
        Box::pin(async {
            let mut rejected = RejectingHookRecord {
                id: 0,
                name: "rejected in managed transaction".to_string(),
            };
            let caught = rejected.save().await;
            assert!(matches!(caught, Err(Error::Validation(_))), "{caught:?}");
            assert_eq!(rejected.id, 0, "failed after_save must restore the ID");
            let mut kept = RejectingHookRecord {
                id: 0,
                name: "kept in managed transaction".to_string(),
            };
            kept.save().await
        })
    })
    .await
    .expect("managed transaction continues after a caught save failure");
    assert_eq!(named_rows("rejected in managed transaction").await, 0);
    assert_eq!(named_rows("kept in managed transaction").await, 1);

    let mut transaction = Orm::begin_transaction()
        .await
        .expect("caller-owned transaction");
    let mut rejected = RejectingHookRecord {
        id: 0,
        name: "rejected in caller transaction".to_string(),
    };
    assert!(rejected.save_with_tx(&mut transaction).await.is_err());
    assert_eq!(rejected.id, 0, "failed after_save must restore the ID");
    let mut kept = RejectingHookRecord {
        id: 0,
        name: "kept in caller transaction".to_string(),
    };
    kept.save_with_tx(&mut transaction)
        .await
        .expect("save after a caught failure");
    transaction
        .commit()
        .await
        .expect("commit caller-owned transaction");
    assert_eq!(named_rows("rejected in caller transaction").await, 0);
    assert_eq!(named_rows("kept in caller transaction").await, 1);
}
