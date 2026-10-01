//! `force_delete()` and `restore()` run the same hook, observer, audit and
//! post-commit pipeline as `delete()`/`save()`.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rullst_orm::audit::{AuditContext, with_audit_context};
use rullst_orm::{Error, FromRow, ModelCommittedEvent, Orm, SearchEngine, set_search_engine};
use serde_json::Value;

static BEFORE_DELETE: AtomicUsize = AtomicUsize::new(0);
static AFTER_DELETE: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(
    table = "trash_customers",
    auditable,
    searchable,
    before_delete = "guard_delete",
    after_delete = "count_deleted"
)]
struct TrashCustomer {
    id: i32,
    name: String,
    deleted_at: Option<String>,
}

impl TrashCustomer {
    async fn guard_delete(&self) -> Result<(), Error> {
        if self.name == "has open invoices" {
            return Err(Error::Validation("customer has open invoices".to_string()));
        }
        BEFORE_DELETE.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn count_deleted(&self) -> Result<(), Error> {
        AFTER_DELETE.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

type Log = Arc<Mutex<Vec<String>>>;

struct RecordingObserver(Log);

#[rullst_orm::async_trait]
impl TrashCustomerObserver for RecordingObserver {
    async fn deleting(&self, model: &TrashCustomer) -> Result<(), Error> {
        self.0
            .lock()
            .unwrap()
            .push(format!("deleting:{}", model.id));
        Ok(())
    }
    async fn deleted(&self, model: &TrashCustomer) -> Result<(), Error> {
        self.0.lock().unwrap().push(format!("deleted:{}", model.id));
        Ok(())
    }
    async fn updated(&self, model: &TrashCustomer) -> Result<(), Error> {
        let state = if model.deleted_at.is_none() {
            "active"
        } else {
            "trashed"
        };
        self.0
            .lock()
            .unwrap()
            .push(format!("updated:{}:{state}", model.id));
        Ok(())
    }
    async fn saved(&self, model: &TrashCustomer) -> Result<(), Error> {
        self.0.lock().unwrap().push(format!("saved:{}", model.id));
        Ok(())
    }
    async fn committed(&self, event: &ModelCommittedEvent) -> Result<(), Error> {
        let payload: Value = serde_json::from_str(&event.payload)?;
        let state = if payload["deleted_at"].is_null() {
            "active"
        } else {
            "trashed"
        };
        self.0.lock().unwrap().push(format!(
            "committed:{}:{}:{state}",
            event.operation.as_str(),
            event.id
        ));
        Ok(())
    }
}

struct RecordingEngine(Log);

#[rullst_orm::async_trait]
impl SearchEngine for RecordingEngine {
    async fn update(&self, _table: &str, id: i32, payload: Value) -> Result<(), Error> {
        let state = if payload["deleted_at"].is_null() {
            "active"
        } else {
            "trashed"
        };
        self.0
            .lock()
            .unwrap()
            .push(format!("scout-update:{id}:{state}"));
        Ok(())
    }
    async fn delete(&self, table: &str, id: i32) -> Result<(), Error> {
        let query = format!("SELECT COUNT(*) FROM {table} WHERE id = ?");
        let rows: (i64,) = sqlx::query_as(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .fetch_one(Orm::pool()?)
            .await?;
        let state = if rows.0 == 0 { "gone" } else { "present" };
        self.0
            .lock()
            .unwrap()
            .push(format!("scout-delete:{id}:{state}"));
        Ok(())
    }
    async fn search(&self, _table: &str, _query: &str) -> Result<Vec<i32>, Error> {
        Ok(Vec::new())
    }
}

fn take(log: &Log) -> Vec<String> {
    std::mem::take(&mut *log.lock().unwrap())
}

async fn audit_events(id: i32) -> Vec<(String, Option<String>, Option<String>)> {
    sqlx::query_as(
        "SELECT event, old_values, new_values FROM rullst_audits WHERE model_type = ? AND model_id = ? ORDER BY id",
    )
    .bind("trash_customers")
    .bind(id)
    .fetch_all(Orm::pool().unwrap())
    .await
    .expect("read audit trail")
}

async fn row_state(id: i32) -> Option<Option<String>> {
    sqlx::query_as::<_, (Option<String>,)>("SELECT deleted_at FROM trash_customers WHERE id = ?")
        .bind(id)
        .fetch_optional(Orm::pool().unwrap())
        .await
        .expect("read raw row")
        .map(|row| row.0)
}

async fn create(name: &str, context: &AuditContext) -> TrashCustomer {
    let mut customer = TrashCustomer {
        id: 0,
        name: name.to_string(),
        deleted_at: None,
    };
    with_audit_context(context.clone(), customer.save())
        .await
        .expect("create customer");
    customer
}

async fn trashed(id: i32) -> TrashCustomer {
    TrashCustomer::query()
        .with_trashed()
        .where_id(id)
        .first()
        .await
        .expect("read trashed customer")
        .expect("trashed customer exists")
}

#[tokio::test]
async fn restore_and_force_delete_run_the_mutation_lifecycle() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-trash-lifecycle-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    Orm::init(&format!("sqlite:{}?mode=rwc", database_path.display()))
        .await
        .expect("initialize SQLite lifecycle database");
    sqlx::query(
        "CREATE TABLE trash_customers (id INTEGER PRIMARY KEY, name TEXT NOT NULL, deleted_at TEXT)",
    )
    .execute(Orm::pool().unwrap())
    .await
    .expect("create customers");
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create audit table");
    let log: Log = Arc::default();
    TrashCustomer::observe(Arc::new(RecordingObserver(log.clone())));
    set_search_engine(RecordingEngine(log.clone())).expect("configure Scout once");
    let context = AuditContext::system("trash-lifecycle").expect("audit context");

    // restore(): observers see the re-read active row; audit, committed and
    // Scout effects follow the update pipeline.
    let customer = create("restorable", &context).await;
    with_audit_context(context.clone(), customer.delete())
        .await
        .expect("soft delete");
    take(&log);
    let deleted = trashed(customer.id).await;
    with_audit_context(context.clone(), deleted.restore())
        .await
        .expect("restore");
    assert_eq!(row_state(customer.id).await, Some(None));
    let id = customer.id;
    assert_eq!(
        take(&log),
        vec![
            format!("updated:{id}:active"),
            format!("saved:{id}"),
            format!("committed:updated:{id}:active"),
            format!("scout-update:{id}:active"),
        ]
    );
    let restored_audit = audit_events(id).await.pop().expect("restore audit row");
    assert_eq!(restored_audit.0, "restored");
    let new_values: Value = serde_json::from_str(restored_audit.2.as_deref().unwrap()).unwrap();
    assert!(new_values["deleted_at"].is_null());
    let old_values: Value = serde_json::from_str(restored_audit.1.as_deref().unwrap()).unwrap();
    assert!(!old_values["deleted_at"].is_null());

    // A rolled-back restore leaves the row trashed and discards its effects.
    with_audit_context(context.clone(), customer.delete())
        .await
        .expect("soft delete again");
    take(&log);
    let deleted = trashed(id).await;
    let rolled_back = with_audit_context(
        context.clone(),
        Orm::transaction(move |_| {
            Box::pin(async move {
                deleted.restore().await?;
                Err::<(), Error>(Error::Validation("abort restore".to_string()))
            })
        }),
    )
    .await;
    assert!(rolled_back.is_err());
    assert!(matches!(row_state(id).await, Some(Some(_))));
    let pending = take(&log);
    assert!(
        pending
            .iter()
            .all(|entry| !entry.starts_with("committed") && !entry.starts_with("scout")),
        "{pending:?}"
    );

    // force_delete(): a before_delete veto keeps the row.
    let guarded = create("has open invoices", &context).await;
    take(&log);
    let vetoed = with_audit_context(context.clone(), guarded.force_delete()).await;
    assert!(matches!(vetoed, Err(Error::Validation(_))), "{vetoed:?}");
    assert!(row_state(guarded.id).await.is_some());
    assert!(take(&log).is_empty());

    // force_delete() without an audit context fails like delete().
    let erased = trashed(id).await;
    let unaudited = erased.force_delete().await;
    assert!(
        matches!(unaudited, Err(Error::Validation(_))),
        "{unaudited:?}"
    );
    assert!(row_state(id).await.is_some());
    take(&log);

    // force_delete(): hooks, observers, audit and post-commit Scout removal.
    let before = BEFORE_DELETE.load(Ordering::SeqCst);
    let after = AFTER_DELETE.load(Ordering::SeqCst);
    with_audit_context(context.clone(), erased.force_delete())
        .await
        .expect("force delete");
    assert_eq!(row_state(id).await, None);
    assert_eq!(BEFORE_DELETE.load(Ordering::SeqCst), before + 1);
    assert_eq!(AFTER_DELETE.load(Ordering::SeqCst), after + 1);
    assert_eq!(
        take(&log),
        vec![
            format!("deleting:{id}"),
            format!("deleted:{id}"),
            format!("committed:deleted:{id}:trashed"),
            format!("scout-delete:{id}:gone"),
        ]
    );
    let erased_audit = audit_events(id)
        .await
        .pop()
        .expect("force delete audit row");
    assert_eq!(erased_audit.0, "force_deleted");
    assert!(erased_audit.1.is_some() && erased_audit.2.is_none());

    // delete() of a trashed row and restore() of a live row match no row:
    // the deletion time, audit trail and post-commit effects stay as they were.
    let twice = create("deleted twice", &context).await;
    with_audit_context(context.clone(), twice.delete())
        .await
        .expect("first soft delete");
    let stamped = row_state(twice.id).await;
    let audits = audit_events(twice.id).await.len();
    take(&log);
    let again = with_audit_context(context.clone(), twice.delete()).await;
    assert!(matches!(again, Err(Error::RecordNotFound)), "{again:?}");
    assert_eq!(row_state(twice.id).await, stamped);
    assert_eq!(audit_events(twice.id).await.len(), audits);
    let id = twice.id;
    assert_eq!(take(&log), vec![format!("deleting:{id}")]);
    let live = create("never deleted", &context).await;
    take(&log);
    with_audit_context(context.clone(), live.restore())
        .await
        .expect("restoring a live row is a no-op");
    assert_eq!(row_state(live.id).await, Some(None));
    assert_eq!(audit_events(live.id).await.len(), 1, "only the creation");
    assert!(take(&log).is_empty());

    // deleted/restored/force_deleted entries record the persisted row, not
    // unsaved edits on the caller's handle.
    let mut edited = create("persisted name", &context).await;
    let old_name = |entry: Option<(String, Option<String>, Option<String>)>| {
        let entry = entry.expect("audit row");
        let old: Value = serde_json::from_str(entry.1.as_deref().expect("old values")).unwrap();
        (entry.0, old["name"].as_str().map(str::to_string))
    };
    edited.name = "unsaved delete edit".to_string();
    with_audit_context(context.clone(), edited.delete())
        .await
        .expect("delete an edited handle");
    assert_eq!(
        old_name(audit_events(edited.id).await.pop()),
        ("deleted".to_string(), Some("persisted name".to_string()))
    );
    let mut restorable = trashed(edited.id).await;
    restorable.name = "unsaved restore edit".to_string();
    with_audit_context(context.clone(), restorable.restore())
        .await
        .expect("restore an edited handle");
    assert_eq!(
        old_name(audit_events(edited.id).await.pop()),
        ("restored".to_string(), Some("persisted name".to_string()))
    );
    let mut erasable = trashed(edited.id).await;
    erasable.name = "unsaved erase edit".to_string();
    with_audit_context(context.clone(), erasable.force_delete())
        .await
        .expect("force delete an edited handle");
    assert_eq!(
        old_name(audit_events(edited.id).await.pop()),
        (
            "force_deleted".to_string(),
            Some("persisted name".to_string())
        )
    );

    Orm::pool().unwrap().close().await;
    let _ = std::fs::remove_file(database_path);
}
