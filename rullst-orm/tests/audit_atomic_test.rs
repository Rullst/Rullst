#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "audit_atomic_accounts", auditable)]
struct AuditedAccount {
    pub id: i32,
    pub name: String,
}

#[tokio::test]
async fn generated_audits_share_the_model_transaction_and_fail_closed() {
    let database_path =
        std::env::temp_dir().join(format!("rullst-audit-atomic-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&database_path);
    let database_url = format!("sqlite:{}?mode=rwc", database_path.to_string_lossy());
    Orm::init(&database_url)
        .await
        .expect("initialize isolated SQLite ORM");
    Schema::create("audit_atomic_accounts", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
    })
    .await
    .expect("create audited model table");
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create audit table");

    let pool = Orm::pool().expect("ORM should be initialized");
    let direct_audit_rollback = Orm::transaction(|_| {
        Box::pin(async move {
            rullst_orm::audit::log_audit(
                "audit_atomic_accounts",
                99,
                "created",
                None,
                Some(r#"{"name":"rolled back"}"#.to_string()),
            )
            .await?;
            Err::<(), rullst_orm::Error>(rullst_orm::Error::Validation(
                "force direct audit rollback".to_string(),
            ))
        })
    })
    .await;
    assert!(direct_audit_rollback.is_err());
    assert_eq!(audit_count(pool).await, 0);

    let mut account = AuditedAccount {
        id: 0,
        name: "committed".to_string(),
    };
    account.save().await.expect("save audited account");
    assert_eq!(audit_count(pool).await, 1);

    let mut update_tx = pool.begin().await.expect("begin audited update");
    account.name = "rolled back update".to_string();
    account
        .save_with_tx(&mut update_tx)
        .await
        .expect("update and audit in explicit transaction");
    let audit_count_inside_update: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_audits")
        .fetch_one(&mut *update_tx)
        .await
        .expect("count update audit inside transaction");
    assert_eq!(audit_count_inside_update.0, 2);
    update_tx.rollback().await.expect("rollback audited update");

    let persisted_name: (String,) =
        sqlx::query_as("SELECT name FROM audit_atomic_accounts WHERE id = ?")
            .bind(account.id)
            .fetch_one(pool)
            .await
            .expect("read account after update rollback");
    assert_eq!(persisted_name.0, "committed");
    assert_eq!(audit_count(pool).await, 1);
    account.name = persisted_name.0;

    let mut delete_tx = pool.begin().await.expect("begin audited delete");
    account
        .delete_with_tx(&mut delete_tx)
        .await
        .expect("delete and audit in explicit transaction");
    let rows_inside_delete: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_atomic_accounts")
        .fetch_one(&mut *delete_tx)
        .await
        .expect("count rows inside delete transaction");
    let audits_inside_delete: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_audits")
        .fetch_one(&mut *delete_tx)
        .await
        .expect("count delete audit inside transaction");
    assert_eq!(rows_inside_delete.0, 0);
    assert_eq!(audits_inside_delete.0, 2);
    delete_tx.rollback().await.expect("rollback audited delete");
    assert_eq!(
        AuditedAccount::query().count().await.expect("count rows"),
        1
    );
    assert_eq!(audit_count(pool).await, 1);

    let scoped_result = Orm::transaction(|_| {
        Box::pin(async move {
            let mut transient = AuditedAccount {
                id: 0,
                name: "task-scoped rollback".to_string(),
            };
            transient.save().await?;
            Err::<(), rullst_orm::Error>(rullst_orm::Error::Validation(
                "force task-scoped rollback".to_string(),
            ))
        })
    })
    .await;
    assert!(scoped_result.is_err());
    assert_eq!(
        AuditedAccount::query().count().await.expect("count rows"),
        1
    );
    assert_eq!(audit_count(pool).await, 1);

    Schema::drop_if_exists("rullst_audits")
        .await
        .expect("drop audit table before fail-closed check");

    let mut delete_tx = pool.begin().await.expect("begin fail-closed delete");
    let delete_error = account
        .delete_with_tx(&mut delete_tx)
        .await
        .expect_err("missing audit table must reject the delete");
    assert!(matches!(delete_error, rullst_orm::Error::DatabaseError(_)));
    delete_tx
        .commit()
        .await
        .expect("outer transaction remains usable after savepoint rollback");
    assert_eq!(
        AuditedAccount::query().count().await.expect("count rows"),
        1
    );

    let scoped_commit = Orm::transaction(|_| {
        Box::pin(async move {
            let mut transient = AuditedAccount {
                id: 0,
                name: "caught task-scoped failure".to_string(),
            };
            let save_error = transient
                .save()
                .await
                .expect_err("task-scoped audit failure must reject save");
            assert!(matches!(save_error, rullst_orm::Error::DatabaseError(_)));
            assert_eq!(transient.id, 0);
            Ok::<(), rullst_orm::Error>(())
        })
    })
    .await;
    assert!(scoped_commit.is_ok());
    assert_eq!(
        AuditedAccount::query().count().await.expect("count rows"),
        1
    );

    let mut rejected = AuditedAccount {
        id: 0,
        name: "must roll back".to_string(),
    };
    let save_error = rejected
        .save()
        .await
        .expect_err("missing audit table must reject the model write");
    assert!(matches!(save_error, rullst_orm::Error::DatabaseError(_)));
    assert_eq!(
        rejected.id, 0,
        "rolled-back insert must restore the model id"
    );
    assert_eq!(
        AuditedAccount::query().count().await.expect("count rows"),
        1
    );

    let _ = std::fs::remove_file(database_path);
}

async fn audit_count(pool: &rullst_orm::RullstPool) -> i64 {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_audits")
        .fetch_one(pool)
        .await
        .expect("count audits");
    count.0
}
