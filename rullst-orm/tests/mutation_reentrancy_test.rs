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
}
