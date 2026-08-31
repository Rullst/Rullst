#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{Error, FromRow, ModelCommittedEvent, Orm};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "audit_atomic_accounts", auditable)]
struct AuditedAccount {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "audit_revision_accounts", auditable)]
struct RevisionAccount {
    pub id: i32,
    pub name: String,
    #[orm(masked)]
    pub password: String,
}

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(
    table = "audit_tenant_revision_accounts",
    auditable,
    tenant_column = "tenant_id"
)]
struct TenantRevisionAccount {
    pub id: i32,
    pub tenant_id: String,
    pub name: String,
}

struct AuditCommitObserver {
    calls: Arc<AtomicUsize>,
}

#[rullst_orm::async_trait]
impl AuditedAccountObserver for AuditCommitObserver {
    async fn committed(&self, _: &ModelCommittedEvent) -> Result<(), Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
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
    Schema::create("audit_revision_accounts", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
        table.string("password").not_null();
    })
    .await
    .expect("create revision model table");
    Schema::create("audit_tenant_revision_accounts", |table: &mut Blueprint| {
        table.id();
        table.string("tenant_id").not_null();
        table.string("name").not_null();
    })
    .await
    .expect("create tenant revision model table");
    let pool = Orm::pool().expect("ORM should be initialized");
    sqlx::query(
        "CREATE TABLE rullst_audits (id INTEGER PRIMARY KEY AUTOINCREMENT, model_type TEXT NOT NULL, model_id INTEGER NOT NULL, event TEXT NOT NULL, old_values TEXT, new_values TEXT, created_at DATETIME DEFAULT CURRENT_TIMESTAMP)",
    )
    .execute(pool)
    .await
    .expect("create legacy audit table");
    sqlx::query(
        "INSERT INTO rullst_audits (model_type, model_id, event, new_values) VALUES (?, ?, ?, ?)",
    )
    .bind("legacy_accounts")
    .bind(7_i32)
    .bind("updated")
    .bind(r#"{"name":"legacy"}"#)
    .execute(pool)
    .await
    .expect("insert legacy audit row");
    rullst_orm::audit::create_audit_table()
        .await
        .expect("upgrade audit table");
    let migrated_legacy: (String, String, i32, Option<String>) = sqlx::query_as(
        "SELECT actor_kind, actor_id, format_version, restore_patch FROM rullst_audits WHERE model_type = ?",
    )
    .bind("legacy_accounts")
    .fetch_one(pool)
    .await
    .expect("read migrated legacy row");
    assert_eq!(migrated_legacy.0, "legacy");
    assert_eq!(migrated_legacy.1, "unknown");
    assert_eq!(migrated_legacy.2, 1);
    assert_eq!(migrated_legacy.3, None);
    sqlx::query("DELETE FROM rullst_audits")
        .execute(pool)
        .await
        .expect("clear legacy fixture");

    let mut missing_context = AuditedAccount {
        id: 0,
        name: "missing context".to_string(),
    };
    let missing_error = missing_context
        .save()
        .await
        .expect_err("auditable save must require an actor");
    assert!(matches!(missing_error, rullst_orm::Error::Validation(_)));
    assert_eq!(missing_context.id, 0);
    assert_eq!(audit_count(pool).await, 0);

    let context = rullst_orm::audit::AuditContext::user("operator-42")
        .expect("valid actor")
        .with_correlation_id("request-7")
        .expect("valid correlation");
    rullst_orm::audit::with_audit_context(context, async {
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
        let identity: (String, String, Option<String>) = sqlx::query_as(
            "SELECT actor_kind, actor_id, correlation_id FROM rullst_audits WHERE model_type = ? AND model_id = ? ORDER BY id DESC LIMIT 1",
        )
        .bind("audit_atomic_accounts")
        .bind(account.id)
        .fetch_one(pool)
        .await
        .expect("read audit identity");
        assert_eq!(identity.0, "user");
        assert_eq!(identity.1, "operator-42");
        assert_eq!(identity.2.as_deref(), Some("request-7"));

        let mut update_tx = pool.begin().await.expect("begin audited update");
        account.name = "rolled back update".to_string();
        account
            .save_with_tx(&mut update_tx)
            .await
            .expect("update and audit in explicit transaction");
        let audit_count_inside_update: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM rullst_audits")
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
        let rows_inside_delete: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM audit_atomic_accounts")
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

        let mut revision_account = RevisionAccount {
            id: 0,
            name: "before".to_string(),
            password: "initial-secret".to_string(),
        };
        revision_account
            .save()
            .await
            .expect("create revision account");
        let created_payload: (String,) = sqlx::query_as(
            "SELECT new_values FROM rullst_audits WHERE model_type = ? AND model_id = ? AND event = 'created' ORDER BY id DESC LIMIT 1",
        )
        .bind("audit_revision_accounts")
        .bind(revision_account.id)
        .fetch_one(pool)
        .await
        .expect("read redacted creation audit");
        assert!(!created_payload.0.contains("initial-secret"));
        assert!(created_payload.0.contains("***"));
        revision_account.name = "after".to_string();
        revision_account
            .save()
            .await
            .expect("create restorable revision");
        let restorable_audit_id = latest_update_audit_id(pool, revision_account.id).await;
        revision_account = revision_account
            .restore_revision(restorable_audit_id, "operator rollback")
            .await
            .expect("restore bounded revision");
        assert_eq!(revision_account.name, "before");
        let persisted_restore: (String, String) = sqlx::query_as(
            "SELECT name, password FROM audit_revision_accounts WHERE id = ?",
        )
        .bind(revision_account.id)
        .fetch_one(pool)
        .await
        .expect("read restored account");
        assert_eq!(persisted_restore.0, "before");
        assert_eq!(persisted_restore.1, "initial-secret");
        let restore_metadata: (Option<i32>, Option<String>, String) = sqlx::query_as(
            "SELECT reverted_audit_id, reason, actor_id FROM rullst_audits WHERE model_type = ? AND model_id = ? ORDER BY id DESC LIMIT 1",
        )
        .bind("audit_revision_accounts")
        .bind(revision_account.id)
        .fetch_one(pool)
        .await
        .expect("read restore metadata");
        assert_eq!(restore_metadata.0, Some(restorable_audit_id));
        assert_eq!(restore_metadata.1.as_deref(), Some("operator rollback"));
        assert_eq!(restore_metadata.2, "operator-42");

        revision_account.password = "rotated-secret".to_string();
        revision_account
            .save()
            .await
            .expect("save redacted revision");
        let redacted_audit_id = latest_update_audit_id(pool, revision_account.id).await;
        let redacted_payload: (String, String, String) = sqlx::query_as(
            "SELECT old_values, new_values, restore_patch FROM rullst_audits WHERE id = ?",
        )
        .bind(redacted_audit_id)
        .fetch_one(pool)
        .await
        .expect("read redacted audit");
        assert!(!redacted_payload.0.contains("initial-secret"));
        assert!(!redacted_payload.1.contains("rotated-secret"));
        assert!(!redacted_payload.2.contains("initial-secret"));
        assert!(!redacted_payload.2.contains("rotated-secret"));
        let redacted_restore = revision_account
            .restore_revision(redacted_audit_id, "unsafe rollback")
            .await;
        assert!(matches!(
            redacted_restore,
            Err(rullst_orm::Error::Validation(_))
        ));
        let persisted_password: (String,) =
            sqlx::query_as("SELECT password FROM audit_revision_accounts WHERE id = ?")
                .bind(revision_account.id)
                .fetch_one(pool)
                .await
                .expect("read password after refused restore");
        assert_eq!(persisted_password.0, "rotated-secret");

        revision_account.name = "revision target".to_string();
        revision_account
            .save()
            .await
            .expect("save stale revision target");
        let stale_audit_id = latest_update_audit_id(pool, revision_account.id).await;
        revision_account.name = "later state".to_string();
        revision_account
            .save()
            .await
            .expect("save later state");
        let stale_restore = revision_account
            .restore_revision(stale_audit_id, "stale rollback")
            .await;
        assert!(matches!(
            stale_restore,
            Err(rullst_orm::Error::Validation(_))
        ));
        let persisted_name: (String,) =
            sqlx::query_as("SELECT name FROM audit_revision_accounts WHERE id = ?")
                .bind(revision_account.id)
                .fetch_one(pool)
                .await
                .expect("read name after refused stale restore");
        assert_eq!(persisted_name.0, "later state");

        let mut tenant_account = TenantRevisionAccount {
            id: 0,
            tenant_id: "forged".to_string(),
            name: "tenant before".to_string(),
        };
        rullst_orm::with_tenant("tenant-a", tenant_account.save())
            .await
            .expect("create tenant revision account");
        tenant_account.name = "tenant after".to_string();
        rullst_orm::with_tenant("tenant-a", tenant_account.save())
            .await
            .expect("save tenant revision");
        let tenant_audit_id: (i32,) = sqlx::query_as(
            "SELECT id FROM rullst_audits WHERE model_type = ? AND model_id = ? AND event = 'updated' ORDER BY id DESC LIMIT 1",
        )
        .bind("audit_tenant_revision_accounts")
        .bind(tenant_account.id)
        .fetch_one(pool)
        .await
        .expect("read tenant revision audit");
        let cross_tenant_restore = rullst_orm::with_tenant(
            "tenant-b",
            tenant_account.restore_revision(tenant_audit_id.0, "cross-tenant rollback"),
        )
        .await;
        assert!(matches!(
            cross_tenant_restore,
            Err(rullst_orm::Error::Validation(_))
        ));
        tenant_account = rullst_orm::with_tenant(
            "tenant-a",
            tenant_account.restore_revision(tenant_audit_id.0, "tenant rollback"),
        )
        .await
        .expect("restore within tenant boundary");
        assert_eq!(tenant_account.name, "tenant before");
        let tenant_metadata: (Option<String>, Option<i32>) = sqlx::query_as(
            "SELECT tenant_key, reverted_audit_id FROM rullst_audits WHERE model_type = ? AND model_id = ? ORDER BY id DESC LIMIT 1",
        )
        .bind("audit_tenant_revision_accounts")
        .bind(tenant_account.id)
        .fetch_one(pool)
        .await
        .expect("read tenant restore metadata");
        assert_eq!(tenant_metadata.0.as_deref(), Some("string:tenant-a"));
        assert_eq!(tenant_metadata.1, Some(tenant_audit_id.0));

        let failed_operation_callbacks = Arc::new(AtomicUsize::new(0));
        AuditedAccount::observe(Arc::new(AuditCommitObserver {
            calls: failed_operation_callbacks.clone(),
        }));

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
        assert_eq!(failed_operation_callbacks.load(Ordering::SeqCst), 0);
    })
    .await;

    let _ = std::fs::remove_file(database_path);
}

async fn audit_count(pool: &rullst_orm::RullstPool) -> i64 {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_audits")
        .fetch_one(pool)
        .await
        .expect("count audits");
    count.0
}

async fn latest_update_audit_id(pool: &rullst_orm::RullstPool, model_id: i32) -> i32 {
    let row: (i32,) = sqlx::query_as(
        "SELECT id FROM rullst_audits WHERE model_type = ? AND model_id = ? AND event = 'updated' ORDER BY id DESC LIMIT 1",
    )
    .bind("audit_revision_accounts")
    .bind(model_id)
    .fetch_one(pool)
    .await
    .expect("read latest update audit");
    row.0
}
