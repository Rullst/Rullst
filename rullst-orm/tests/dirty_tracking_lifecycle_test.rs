//! The save lifecycle of models with a `RowSnapshot` field: hooks, observers,
//! tenant scope, auditing and failed saves.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::audit::{AuditContext, with_audit_context};
use rullst_orm::{Error, FromRow, Orm, RowSnapshot, with_tenant};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "flaky_posts", after_save = "reject_boom")]
struct FlakyPost {
    id: i32,
    title: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

impl FlakyPost {
    async fn reject_boom(&mut self) -> Result<(), Error> {
        if self.title == "boom" {
            return Err(Error::Validation("boom rejected".to_string()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "observed_posts")]
struct ObservedPost {
    id: i32,
    title: String,
    body: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

#[derive(Default)]
struct Counts {
    saving: AtomicUsize,
    updating: AtomicUsize,
    updated: AtomicUsize,
    saved: AtomicUsize,
}

struct CountingObserver(Arc<Counts>);

#[rullst_orm::async_trait]
impl ObservedPostObserver for CountingObserver {
    async fn saving(&self, model: &mut ObservedPost) -> Result<(), Error> {
        self.0.saving.fetch_add(1, Ordering::SeqCst);
        if model.title == "normalize me" {
            model.body = "normalized by observer".to_string();
        }
        Ok(())
    }
    async fn updating(&self, model: &mut ObservedPost) -> Result<(), Error> {
        self.0.updating.fetch_add(1, Ordering::SeqCst);
        if model.title == "stamp on update" {
            model.body = "stamped by updating".to_string();
        }
        Ok(())
    }
    async fn updated(&self, _model: &ObservedPost) -> Result<(), Error> {
        self.0.updated.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn saved(&self, _model: &ObservedPost) -> Result<(), Error> {
        self.0.saved.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "tenant_notes", tenant_column = "org")]
struct TenantNote {
    id: i32,
    org: String,
    title: String,
    body: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "audited_posts", auditable)]
struct AuditedPost {
    id: i32,
    title: String,
    body: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

async fn execute(sql: &str) {
    rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.to_string()))
        .execute(Orm::pool().expect("pool"))
        .await
        .unwrap_or_else(|error| panic!("{sql}: {error}"));
}

async fn stored_title(table: &str, id: i32) -> String {
    let row: (String,) = rullst_orm::_sqlx::query_as(rullst_orm::_sqlx::AssertSqlSafe(format!(
        "SELECT title FROM {table} WHERE id = ?"
    )))
    .bind(id)
    .fetch_one(Orm::pool().expect("pool"))
    .await
    .expect("read title");
    row.0
}

async fn update_audits(id: i32) -> Vec<(String, String)> {
    rullst_orm::_sqlx::query_as(
        "SELECT old_values, new_values FROM rullst_audits WHERE model_type = 'audited_posts' AND model_id = ? AND event = 'updated' ORDER BY id",
    )
    .bind(id)
    .fetch_all(Orm::pool().expect("pool"))
    .await
    .expect("read audits")
}

#[tokio::test]
async fn tracked_saves_keep_the_lifecycle_contract() {
    Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .expect("initialize SQLite");
    for table in ["flaky_posts", "audited_posts"] {
        execute(&format!(
            "CREATE TABLE {table} (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, body TEXT NOT NULL DEFAULT '')"
        ))
        .await;
    }
    execute("CREATE TABLE observed_posts (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, body TEXT NOT NULL)").await;
    execute("CREATE TABLE tenant_notes (id INTEGER PRIMARY KEY AUTOINCREMENT, org TEXT NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL)").await;
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create audit table");

    failed_saves_stay_dirty().await;
    observers_see_only_writes().await;
    tenant_saves_stay_scoped().await;
    let context = AuditContext::system("dirty-tracking").expect("audit context");
    with_audit_context(context, audits_record_only_written_columns()).await;
}

async fn failed_saves_stay_dirty() {
    let mut post = FlakyPost {
        id: 0,
        title: "fine".into(),
        snapshot: RowSnapshot::default(),
    };
    post.save().await.expect("insert");
    post.title = "boom".into();
    let failed = post.save().await;
    assert!(matches!(failed, Err(Error::Validation(_))), "{failed:?}");
    assert_eq!(stored_title("flaky_posts", post.id).await, "fine");
    // The rolled-back write is still pending.
    assert_eq!(post.dirty_columns(), ["title"]);
    post.title = "recovered".into();
    post.save().await.expect("save after failure");
    assert_eq!(stored_title("flaky_posts", post.id).await, "recovered");
    assert!(!post.is_dirty());

    // A failed insert leaves the record new and untracked.
    let mut new_post = FlakyPost {
        id: 0,
        title: "boom".into(),
        snapshot: RowSnapshot::default(),
    };
    assert!(new_post.save().await.is_err());
    assert_eq!(new_post.id, 0);
    assert!(!new_post.snapshot.is_tracked());
}

async fn observers_see_only_writes() {
    let counts = Arc::new(Counts::default());
    ObservedPost::observe(Arc::new(CountingObserver(counts.clone())));
    let mut post = ObservedPost {
        id: 0,
        title: "observed".into(),
        body: "body".into(),
        snapshot: RowSnapshot::default(),
    };
    post.save().await.expect("insert");
    assert_eq!(counts.saving.load(Ordering::SeqCst), 1);
    assert_eq!(counts.saved.load(Ordering::SeqCst), 1);

    // An unchanged save runs the `saving` observers, then stops.
    post.save().await.expect("unchanged save");
    assert_eq!(counts.saving.load(Ordering::SeqCst), 2);
    assert_eq!(counts.updating.load(Ordering::SeqCst), 0);
    assert_eq!(counts.updated.load(Ordering::SeqCst), 0);
    assert_eq!(counts.saved.load(Ordering::SeqCst), 1);

    // A change made by a `saving` observer is written.
    post.title = "normalize me".into();
    post.save().await.expect("observed save");
    assert_eq!(counts.updating.load(Ordering::SeqCst), 1);
    assert_eq!(counts.updated.load(Ordering::SeqCst), 1);
    assert_eq!(counts.saved.load(Ordering::SeqCst), 2);
    let stored = ObservedPost::find(post.id)
        .await
        .expect("find")
        .expect("stored");
    assert_eq!(stored.body, "normalized by observer");

    // A change made by an `updating` observer is written too.
    post.title = "stamp on update".into();
    post.save().await.expect("stamped save");
    let stored = ObservedPost::find(post.id)
        .await
        .expect("find")
        .expect("stored");
    assert_eq!(stored.title, "stamp on update");
    assert_eq!(stored.body, "stamped by updating");
    assert!(!post.is_dirty());
}

async fn tenant_saves_stay_scoped() {
    let id = with_tenant("acme", async {
        let mut note = TenantNote {
            id: 0,
            org: String::new(),
            title: "t".into(),
            body: "b".into(),
            snapshot: RowSnapshot::default(),
        };
        note.save().await.expect("insert tenant note");
        let mut loaded = TenantNote::find(note.id)
            .await
            .expect("find")
            .expect("tenant note");
        loaded.body = "acme body".into();
        loaded.save().await.expect("tenant save");
        assert!(!loaded.is_dirty());
        note.id
    })
    .await;

    let stored: (String, String) =
        rullst_orm::_sqlx::query_as("SELECT org, body FROM tenant_notes WHERE id = ?")
            .bind(id)
            .fetch_one(Orm::pool().expect("pool"))
            .await
            .expect("read tenant note");
    assert_eq!(stored, ("acme".to_string(), "acme body".to_string()));

    let mut acme_note = with_tenant("acme", async {
        TenantNote::find(id).await.expect("find").expect("note")
    })
    .await;
    // Unchanged without a tenant context: still refused, as before.
    let unscoped = acme_note.save().await;
    assert!(
        matches!(unscoped, Err(Error::Validation(_))),
        "{unscoped:?}"
    );
    // Changed under another tenant: the scoped UPDATE matches no row.
    acme_note.title = "hijack".into();
    let foreign = with_tenant("globex", acme_note.save()).await;
    assert!(matches!(foreign, Err(Error::Validation(_))), "{foreign:?}");
    assert_eq!(stored_title("tenant_notes", id).await, "t");
}

async fn audits_record_only_written_columns() {
    let mut post = AuditedPost {
        id: 0,
        title: "draft".into(),
        body: "body".into(),
        snapshot: RowSnapshot::default(),
    };
    post.save().await.expect("insert audited post");
    let mut first = AuditedPost::find(post.id)
        .await
        .expect("find")
        .expect("post");
    let mut second = first.clone();
    first.title = "title from first".into();
    first.save().await.expect("first save");
    second.body = "body from second".into();
    second.save().await.expect("second save");
    // An unchanged save audits nothing.
    second.save().await.expect("unchanged save");

    let audits = update_audits(post.id).await;
    assert_eq!(audits.len(), 2, "{audits:?}");
    assert!(audits[0].1.contains("title from first"), "{audits:?}");
    assert!(!audits[0].1.contains("body"), "{audits:?}");
    // The second writer did not write the title, so its audit row does not
    // report the first writer's title as its own change.
    assert!(audits[1].1.contains("body from second"), "{audits:?}");
    assert!(!audits[1].0.contains("title"), "{audits:?}");
    assert!(!audits[1].1.contains("title"), "{audits:?}");
    assert_eq!(second.title, "title from first");
}
