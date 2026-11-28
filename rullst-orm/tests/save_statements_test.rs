//! The statements each direct `save()` of a model without hooks, policy,
//! observers or auditing sends to SQLite, whether it opens its own
//! transaction, and what a failed one leaves behind.
//!
//! SQLx logs every statement it executes (`sqlx::query` events) except the
//! transaction control statements, so a transaction that a save opens shows
//! as the `rullst.orm.transaction.begin` span of `Orm::begin_transaction`.
//! `save_lifecycle_statements_test` covers the other models.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod save_statements;

use rullst_orm::{Error, Orm, RowSnapshot};
use save_statements::{Recorder, assert_sent, execute, record, row_count, stored_text};

/// One tracked column, as the Benchmarks `World` model has.
#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "sp_tallies")]
struct Tally {
    id: i32,
    hits: i32,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

/// Two tracked columns, one of them unique.
#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "sp_cards")]
struct Card {
    id: i32,
    code: String,
    title: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

/// A tracked model with a column the database manages.
#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "sp_stamps")]
struct Stamp {
    id: i32,
    title: String,
    #[orm(db_default)]
    created_at: Option<String>,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "sp_ledgers", full_row_save)]
struct Ledger {
    id: i32,
    code: String,
    amount: i32,
}

#[tokio::test(flavor = "current_thread")]
async fn plain_save_paths_send_their_pinned_statements() {
    let recorder = save_statements::sqlite(&[
        "CREATE TABLE sp_tallies (id INTEGER PRIMARY KEY AUTOINCREMENT, hits INTEGER NOT NULL)",
        "CREATE TABLE sp_cards (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT NOT NULL UNIQUE, title TEXT NOT NULL)",
        "CREATE TABLE sp_stamps (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, created_at TEXT DEFAULT CURRENT_TIMESTAMP)",
        "CREATE TABLE sp_ledgers (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT NOT NULL UNIQUE, amount INTEGER NOT NULL)",
    ])
    .await;
    plain_saves(&recorder).await;
    failed_plain_saves(&recorder).await;
}

/// Saves of models without hooks, policy, observers or auditing.
async fn plain_saves(recorder: &Recorder) {
    let mut tally = Tally {
        id: 0,
        hits: 1,
        snapshot: RowSnapshot::default(),
    };
    let (saved, seen) = record(recorder, tally.save()).await;
    saved.expect("insert tally");
    assert_sent(
        &seen,
        1,
        &["INSERT INTO sp_tallies (hits) VALUES (?) RETURNING id"],
        "insert",
    );

    tally.hits = 2;
    let (saved, seen) = record(recorder, tally.save()).await;
    saved.expect("update tally");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_tallies SET hits = ? WHERE id = ? RETURNING *"],
        "tracked update of every column",
    );

    let (saved, seen) = record(recorder, tally.save()).await;
    saved.expect("unchanged tally");
    assert_sent(&seen, 0, &[], "unchanged tracked record");

    let mut card = Card {
        id: 0,
        code: "c-1".into(),
        title: "first".into(),
        snapshot: RowSnapshot::default(),
    };
    card.save().await.expect("insert card");
    card.title = "second".into();
    let (saved, seen) = record(recorder, card.save()).await;
    saved.expect("update one card column");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_cards SET title = ? WHERE id = ? RETURNING *"],
        "tracked update of some columns",
    );
    card.code = "c-2".into();
    card.title = "third".into();
    let (saved, seen) = record(recorder, card.save()).await;
    saved.expect("update both card columns");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_cards SET code = ?, title = ? WHERE id = ? RETURNING *"],
        "tracked update of every column of two",
    );

    let mut stamp = Stamp {
        id: 0,
        title: "draft".into(),
        created_at: None,
        snapshot: RowSnapshot::default(),
    };
    stamp.save().await.expect("insert stamp");
    stamp.title = "final".into();
    let (saved, seen) = record(recorder, stamp.save()).await;
    saved.expect("update stamp");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_stamps SET title = ? WHERE id = ? RETURNING *"],
        "tracked update with a database-managed column",
    );
    assert!(
        stamp.created_at.is_some(),
        "the managed column is refreshed"
    );

    let mut ledger = Ledger {
        id: 0,
        code: "l-1".into(),
        amount: 1,
    };
    let (saved, seen) = record(recorder, ledger.save()).await;
    saved.expect("insert ledger");
    assert_sent(
        &seen,
        1,
        &["INSERT INTO sp_ledgers (code, amount) VALUES (?, ?) RETURNING id"],
        "full-row insert",
    );
    ledger.amount = 2;
    let (saved, seen) = record(recorder, ledger.save()).await;
    saved.expect("update ledger");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_ledgers SET code = ?, amount = ? WHERE id = ?"],
        "full-row update",
    );

    // A query timeout bounds each statement.
    Orm::set_query_timeout(30);
    tally.hits = 3;
    let (saved, seen) = record(recorder, tally.save()).await;
    Orm::set_query_timeout(0);
    saved.expect("update tally under a query timeout");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_tallies SET hits = ? WHERE id = ? RETURNING *"],
        "tracked update under a query timeout",
    );
}

/// A rejected statement or a missing row persists nothing and leaves the
/// handle as it was: still new after an insert, still dirty after an update.
async fn failed_plain_saves(recorder: &Recorder) {
    let mut taken = Card {
        id: 0,
        code: "c-2".into(),
        title: "duplicate".into(),
        snapshot: RowSnapshot::default(),
    };
    let (saved, seen) = record(recorder, taken.save()).await;
    assert!(
        matches!(saved, Err(Error::UniqueViolation { .. })),
        "{saved:?}"
    );
    assert_sent(
        &seen,
        1,
        &["INSERT INTO sp_cards (code, title) VALUES (?, ?) RETURNING id"],
        "insert of a duplicate",
    );
    assert_eq!(taken.id, 0, "a failed insert keeps the record new");
    assert!(!taken.snapshot.is_tracked());

    let mut other = Card {
        id: 0,
        code: "c-3".into(),
        title: "other".into(),
        snapshot: RowSnapshot::default(),
    };
    other.save().await.expect("insert other card");
    other.code = "c-2".into();
    let (saved, seen) = record(recorder, other.save()).await;
    assert!(
        matches!(saved, Err(Error::UniqueViolation { .. })),
        "{saved:?}"
    );
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_cards SET code = ? WHERE id = ? RETURNING *"],
        "update to a duplicate",
    );
    let code = stored_text("SELECT code FROM sp_cards WHERE id = ?", other.id).await;
    assert_eq!(
        code.as_deref(),
        Some("c-3"),
        "the rejected update wrote nothing"
    );
    assert_eq!(other.dirty_columns(), ["code"], "the change stays pending");

    let mut ledger = Ledger {
        id: 0,
        code: "l-1".into(),
        amount: 9,
    };
    let (saved, seen) = record(recorder, ledger.save()).await;
    assert!(
        matches!(saved, Err(Error::UniqueViolation { .. })),
        "{saved:?}"
    );
    assert_eq!(seen.begins, 1, "full-row insert of a duplicate");
    assert_eq!(ledger.id, 0);
    assert_eq!(row_count("sp_ledgers").await, 1);

    let mut vanished = Tally::find(1).await.expect("load").expect("tally 1");
    execute("DELETE FROM sp_tallies WHERE id = 1").await;
    vanished.hits = 40;
    let (saved, seen) = record(recorder, vanished.save()).await;
    assert!(matches!(saved, Err(Error::RecordNotFound)), "{saved:?}");
    assert_sent(
        &seen,
        1,
        &["UPDATE sp_tallies SET hits = ? WHERE id = ? RETURNING *"],
        "update of a deleted row",
    );
    assert_eq!(vanished.id, 1);
    assert_eq!(vanished.dirty_columns(), ["hits"]);
}
