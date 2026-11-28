//! Saves of models with hooks, a policy, observers or auditing, and saves
//! inside `Orm::transaction`, on SQLite: the transaction they keep, the
//! statements they send and what a failure after the write rolls back. See
//! `save_statements_test` for how statements and transactions are recorded.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod save_statements;

use std::sync::Arc;

use rullst_orm::audit::{AuditContext, with_audit_context};
use rullst_orm::{Error, Orm, Policy, RowSnapshot};
use save_statements::{Recorder, assert_sent, record, row_count, stored_text};

/// One tracked column, as the Benchmarks `World` model has.
#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "sp_tallies")]
struct Tally {
    id: i32,
    hits: i32,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "sp_drafts", before_save = "refuse_boom")]
struct Draft {
    id: i32,
    title: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

impl Draft {
    async fn refuse_boom(&mut self) -> Result<(), Error> {
        refuse(&self.title)
    }
}

#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "sp_receipts", after_save = "refuse_boom")]
struct Receipt {
    id: i32,
    title: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

impl Receipt {
    async fn refuse_boom(&mut self) -> Result<(), Error> {
        refuse(&self.title)
    }
}

#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "sp_guarded", policy = "OpenPolicy")]
struct Guarded {
    id: i32,
    title: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

struct OpenPolicy;

#[rullst_orm::async_trait]
impl Policy<Guarded> for OpenPolicy {}

#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "sp_watched")]
struct Watched {
    id: i32,
    title: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

struct RefusingObserver;

#[rullst_orm::async_trait]
impl WatchedObserver for RefusingObserver {
    async fn saved(&self, model: &Watched) -> Result<(), Error> {
        refuse(&model.title)
    }
}

#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "sp_audited", auditable)]
struct Audited {
    id: i32,
    title: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

fn refuse(title: &str) -> Result<(), Error> {
    if title == "boom" {
        return Err(Error::Validation("boom refused".to_string()));
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn lifecycle_save_paths_send_their_pinned_statements() {
    let recorder = save_statements::sqlite(&[
        "CREATE TABLE sp_tallies (id INTEGER PRIMARY KEY AUTOINCREMENT, hits INTEGER NOT NULL)",
        "CREATE TABLE sp_drafts (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL)",
        "CREATE TABLE sp_receipts (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL)",
        "CREATE TABLE sp_guarded (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL)",
        "CREATE TABLE sp_watched (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL)",
        "CREATE TABLE sp_audited (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL)",
    ])
    .await;
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create audit table");
    // Without a query timeout, only what the model declares keeps the
    // transaction.
    Orm::set_query_timeout(0);
    lifecycle_saves(&recorder).await;
    joined_saves(&recorder).await;
}

/// Hooks, policies, observers and auditing keep the save in its transaction:
/// a failure after the write rolls it back.
async fn lifecycle_saves(recorder: &Recorder) {
    let mut draft = Draft {
        id: 0,
        title: "fine".into(),
        snapshot: RowSnapshot::default(),
    };
    draft.save().await.expect("insert draft");
    draft.title = "boom".into();
    let (saved, seen) = record(recorder, draft.save()).await;
    assert!(matches!(saved, Err(Error::Validation(_))), "{saved:?}");
    assert_sent(&seen, 1, &[], "failing before_save hook");
    assert_eq!(draft.dirty_columns(), ["title"]);
    draft.title = "edited".into();
    let (saved, seen) = record(recorder, draft.save()).await;
    saved.expect("save with a before_save hook");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_drafts SET title = ? WHERE id = ?"],
        "before_save hook",
    );

    let mut receipt = Receipt {
        id: 0,
        title: "fine".into(),
        snapshot: RowSnapshot::default(),
    };
    receipt.save().await.expect("insert receipt");
    receipt.title = "boom".into();
    let (saved, seen) = record(recorder, receipt.save()).await;
    assert!(matches!(saved, Err(Error::Validation(_))), "{saved:?}");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_receipts SET title = ? WHERE id = ?"],
        "failing after_save hook",
    );
    let title = stored_text("SELECT title FROM sp_receipts WHERE id = ?", receipt.id).await;
    assert_eq!(title.as_deref(), Some("fine"), "the write rolled back");
    assert_eq!(receipt.dirty_columns(), ["title"]);
    let mut new_receipt = Receipt {
        id: 0,
        title: "boom".into(),
        snapshot: RowSnapshot::default(),
    };
    assert!(new_receipt.save().await.is_err());
    assert_eq!(
        new_receipt.id, 0,
        "the rolled-back insert keeps the record new"
    );
    assert_eq!(row_count("sp_receipts").await, 1);

    let mut guarded = Guarded {
        id: 0,
        title: "fine".into(),
        snapshot: RowSnapshot::default(),
    };
    guarded.save().await.expect("insert guarded");
    guarded.title = "edited".into();
    let (saved, seen) = record(recorder, guarded.save()).await;
    saved.expect("save with a policy");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_guarded SET title = ? WHERE id = ?"],
        "policy",
    );

    Watched::observe(Arc::new(RefusingObserver));
    let mut watched = Watched {
        id: 0,
        title: "fine".into(),
        snapshot: RowSnapshot::default(),
    };
    let (saved, seen) = record(recorder, watched.save()).await;
    saved.expect("insert watched");
    assert_sent(
        &seen,
        1,
        &["INSERT INTO sp_watched (title) VALUES (?) RETURNING id"],
        "insert with an observer",
    );
    watched.title = "boom".into();
    let (saved, seen) = record(recorder, watched.save()).await;
    assert!(matches!(saved, Err(Error::Validation(_))), "{saved:?}");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_watched SET title = ? WHERE id = ?"],
        "failing saved observer",
    );
    let title = stored_text("SELECT title FROM sp_watched WHERE id = ?", watched.id).await;
    assert_eq!(title.as_deref(), Some("fine"), "the write rolled back");

    let context = AuditContext::system("save-statements").expect("audit context");
    let mut audited = Audited {
        id: 0,
        title: "fine".into(),
        snapshot: RowSnapshot::default(),
    };
    with_audit_context(context.clone(), audited.save())
        .await
        .expect("insert audited");
    audited.title = "edited".into();
    let (saved, seen) = record(recorder, with_audit_context(context, audited.save())).await;
    saved.expect("audited update");
    assert_eq!(seen.begins, 1, "auditable save");
    assert_eq!(
        seen.statements.first().map(String::as_str),
        Some("SELECT * FROM sp_audited WHERE id = ?"),
        "an auditable save reads its pre-image first"
    );
}

/// Saves inside `Orm::transaction` run in savepoints of that transaction.
async fn joined_saves(recorder: &Recorder) {
    let mut tally = Tally {
        id: 0,
        hits: 1,
        snapshot: RowSnapshot::default(),
    };
    tally.save().await.expect("insert tally");
    let id = tally.id;
    let (outcome, seen) = record(
        recorder,
        Orm::transaction(move |_| {
            Box::pin(async move {
                tally.hits = 2;
                tally.save().await?;
                tally.hits = 3;
                tally.save().await?;
                Err::<(), Error>(Error::Validation("roll the transaction back".into()))
            })
        }),
    )
    .await;
    assert!(matches!(outcome, Err(Error::Validation(_))), "{outcome:?}");
    assert_sent(
        &seen,
        1,
        &[
            "UPDATE sp_tallies SET hits = ? WHERE id = ?",
            "UPDATE sp_tallies SET hits = ? WHERE id = ?",
        ],
        "saves inside Orm::transaction",
    );
    let stored = Tally::find(id).await.expect("load").expect("stored tally");
    assert_eq!(stored.hits, 1, "the outer rollback discarded both saves");
}
