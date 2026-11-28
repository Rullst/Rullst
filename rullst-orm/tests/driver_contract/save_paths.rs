//! What a direct `save()` sends and what a failed one leaves behind, on every
//! SQL backend: the transactions it opens, its statements (recorded here on
//! PostgreSQL and MySQL/MariaDB; `save_statements_test` records SQLite's),
//! and the stored row and handle after a rejected write, a missing row, a
//! failing `after_save` hook and an outer rollback.

use super::save_recorder::{Recorded, Recorder};
use rullst_orm::schema::Schema;
use rullst_orm::{Error, Orm, RowSnapshot};

#[derive(Debug, Clone, Orm)]
#[orm(table = "contract_save_tallies")]
struct ContractSaveTally {
    id: i32,
    hits: i32,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

#[derive(Debug, Clone, Orm)]
#[orm(table = "contract_save_cards")]
struct ContractSaveCard {
    id: i32,
    code: String,
    title: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

#[derive(Debug, Clone, Orm)]
#[orm(table = "contract_save_ledgers", full_row_save)]
struct ContractSaveLedger {
    id: i32,
    code: String,
    amount: i32,
}

#[derive(Debug, Clone, Orm)]
#[orm(table = "contract_save_receipts", after_save = "refuse_boom")]
struct ContractSaveReceipt {
    id: i32,
    title: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

impl ContractSaveReceipt {
    async fn refuse_boom(&mut self) -> Result<(), Error> {
        if self.title == "boom" {
            return Err(Error::Validation("boom refused".to_string()));
        }
        Ok(())
    }
}

/// The statements one save sends on `driver`, when this backend's
/// statements are recorded (see the module documentation).
struct Expected {
    driver: &'static str,
}

impl Expected {
    fn insert(&self, table: &str, columns: &str) -> Vec<String> {
        let placeholders = vec!["?"; columns.split(", ").count()].join(", ");
        let insert = format!("INSERT INTO {table} ({columns}) VALUES ({placeholders})");
        if self.driver == "mysql" {
            vec![insert]
        } else {
            vec![format!("{insert} RETURNING id")]
        }
    }

    /// A tracked `UPDATE` that reads the stored row back: `RETURNING *`, or
    /// on MySQL/MariaDB a `SELECT` after it.
    fn update_returning(&self, table: &str, assignments: &str) -> Vec<String> {
        let update = format!("UPDATE {table} SET {assignments} WHERE id = ?");
        if self.driver == "mysql" {
            vec![update, format!("SELECT * FROM {table} WHERE id = ?")]
        } else {
            vec![format!("{update} RETURNING *")]
        }
    }

    fn update(&self, table: &str, assignments: &str) -> Vec<String> {
        vec![format!("UPDATE {table} SET {assignments} WHERE id = ?")]
    }

    fn check(&self, seen: &Recorded, begins: usize, statements: &[String], case: &str) {
        let driver = self.driver;
        assert_eq!(seen.begins, begins, "{driver} {case}: transactions opened");
        if driver != "sqlite" {
            assert_eq!(seen.statements, statements, "{driver} {case}: statements");
        }
    }
}

async fn stored_card_code(id: i32) -> Option<String> {
    ContractSaveCard::find(id)
        .await
        .expect("load card")
        .map(|card| card.code)
}

async fn create_tables() {
    Schema::create("contract_save_tallies", |table| {
        table.id();
        table.integer("hits").not_null();
    })
    .await
    .expect("create tally contract table");
    for name in ["contract_save_cards", "contract_save_ledgers"] {
        Schema::create(name, |table| {
            table.id();
            // MySQL/MariaDB cannot make a TEXT column unique.
            table.string("code").not_null().col_type = "VARCHAR(191)".to_string();
            table.unique(&["code"]);
            if name == "contract_save_cards" {
                table.string("title").not_null();
            } else {
                table.integer("amount").not_null();
            }
        })
        .await
        .unwrap_or_else(|error| panic!("create {name}: {error}"));
    }
    Schema::create("contract_save_receipts", |table| {
        table.id();
        table.string("title").not_null();
    })
    .await
    .expect("create receipt contract table");
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    create_tables().await;
    let expected = Expected { driver };
    let (recorder, _guard) = Recorder::install_local(&["contract_save_"]);

    plain_saves(&recorder, &expected).await;
    failed_saves(&recorder, &expected).await;
    hooked_and_joined_saves(&recorder, &expected).await;
}

async fn plain_saves(recorder: &Recorder, expected: &Expected) {
    let mut tally = ContractSaveTally {
        id: 0,
        hits: 1,
        snapshot: RowSnapshot::default(),
    };
    recorder.take();
    tally.save().await.expect("insert tally");
    expected.check(
        &recorder.take(),
        1,
        &expected.insert("contract_save_tallies", "hits"),
        "insert",
    );
    tally.hits = 2;
    tally.save().await.expect("update tally");
    expected.check(
        &recorder.take(),
        1,
        &expected.update_returning("contract_save_tallies", "hits = ?"),
        "tracked update of every column",
    );

    let mut card = ContractSaveCard {
        id: 0,
        code: "card-1".into(),
        title: "first".into(),
        snapshot: RowSnapshot::default(),
    };
    card.save().await.expect("insert card");
    recorder.take();
    card.title = "second".into();
    card.save().await.expect("update one card column");
    expected.check(
        &recorder.take(),
        1,
        &expected.update_returning("contract_save_cards", "title = ?"),
        "tracked update of some columns",
    );
    card.mark_all_dirty();
    card.save().await.expect("full write of a card");
    expected.check(
        &recorder.take(),
        1,
        &expected.update_returning("contract_save_cards", "code = ?, title = ?"),
        "tracked update after mark_all_dirty()",
    );

    let mut ledger = ContractSaveLedger {
        id: 0,
        code: "ledger-1".into(),
        amount: 1,
    };
    ledger.save().await.expect("insert ledger");
    expected.check(
        &recorder.take(),
        1,
        &expected.insert("contract_save_ledgers", "code, amount"),
        "full-row insert",
    );
    ledger.amount = 2;
    ledger.save().await.expect("update ledger");
    expected.check(
        &recorder.take(),
        1,
        &expected.update("contract_save_ledgers", "code = ?, amount = ?"),
        "full-row update",
    );
}

/// A rejected statement or a missing row persists nothing and leaves the
/// handle as it was: still new after an insert, still dirty after an update.
async fn failed_saves(recorder: &Recorder, expected: &Expected) {
    let driver = expected.driver;
    let mut duplicate = ContractSaveCard {
        id: 0,
        code: "card-1".into(),
        title: "duplicate".into(),
        snapshot: RowSnapshot::default(),
    };
    recorder.take();
    let saved = duplicate.save().await;
    assert!(
        matches!(saved, Err(Error::UniqueViolation { .. })),
        "{driver}: {saved:?}"
    );
    expected.check(
        &recorder.take(),
        1,
        &expected.insert("contract_save_cards", "code, title"),
        "insert of a duplicate",
    );
    assert_eq!(
        duplicate.id, 0,
        "{driver}: a failed insert keeps the record new"
    );
    assert!(!duplicate.snapshot.is_tracked(), "{driver}");

    let mut other = ContractSaveCard {
        id: 0,
        code: "card-2".into(),
        title: "other".into(),
        snapshot: RowSnapshot::default(),
    };
    other.save().await.expect("insert other card");
    other.code = "card-1".into();
    recorder.take();
    let saved = other.save().await;
    assert!(
        matches!(saved, Err(Error::UniqueViolation { .. })),
        "{driver}: {saved:?}"
    );
    expected.check(
        &recorder.take(),
        1,
        &expected.update_returning("contract_save_cards", "code = ?")[..1],
        "update to a duplicate",
    );
    assert_eq!(
        stored_card_code(other.id).await.as_deref(),
        Some("card-2"),
        "{driver}: the rejected update wrote nothing"
    );
    assert_eq!(other.dirty_columns(), ["code"], "{driver}: still pending");

    let mut ledger = ContractSaveLedger {
        id: 0,
        code: "ledger-1".into(),
        amount: 9,
    };
    let saved = ledger.save().await;
    assert!(
        matches!(saved, Err(Error::UniqueViolation { .. })),
        "{driver}: {saved:?}"
    );
    assert_eq!(ledger.id, 0, "{driver}");

    let mut vanished = ContractSaveTally {
        id: 0,
        hits: 1,
        snapshot: RowSnapshot::default(),
    };
    vanished.save().await.expect("insert tally");
    let mut vanished = ContractSaveTally::find(vanished.id)
        .await
        .expect("load tally")
        .expect("stored tally");
    ContractSaveTally::find(vanished.id)
        .await
        .expect("load tally")
        .expect("stored tally")
        .delete()
        .await
        .expect("delete tally");
    vanished.hits = 40;
    recorder.take();
    let saved = vanished.save().await;
    assert!(
        matches!(saved, Err(Error::RecordNotFound)),
        "{driver}: {saved:?}"
    );
    expected.check(
        &recorder.take(),
        1,
        &expected.update_returning("contract_save_tallies", "hits = ?")[..1],
        "update of a deleted row",
    );
    assert_eq!(vanished.dirty_columns(), ["hits"], "{driver}");
}

/// A failing `after_save` hook rolls the write back, and saves inside
/// `Orm::transaction` commit or roll back with it.
async fn hooked_and_joined_saves(recorder: &Recorder, expected: &Expected) {
    let driver = expected.driver;
    let mut receipt = ContractSaveReceipt {
        id: 0,
        title: "fine".into(),
        snapshot: RowSnapshot::default(),
    };
    receipt.save().await.expect("insert receipt");
    receipt.title = "boom".into();
    recorder.take();
    let saved = receipt.save().await;
    assert!(
        matches!(saved, Err(Error::Validation(_))),
        "{driver}: {saved:?}"
    );
    expected.check(
        &recorder.take(),
        1,
        &expected.update_returning("contract_save_receipts", "title = ?"),
        "failing after_save hook",
    );
    let stored = ContractSaveReceipt::find(receipt.id)
        .await
        .expect("load receipt")
        .expect("stored receipt");
    assert_eq!(stored.title, "fine", "{driver}: the write rolled back");
    assert_eq!(receipt.dirty_columns(), ["title"], "{driver}");
    let mut new_receipt = ContractSaveReceipt {
        id: 0,
        title: "boom".into(),
        snapshot: RowSnapshot::default(),
    };
    assert!(new_receipt.save().await.is_err(), "{driver}");
    assert_eq!(
        new_receipt.id, 0,
        "{driver}: the rolled-back insert keeps the record new"
    );
    let receipts = ContractSaveReceipt::query()
        .get()
        .await
        .expect("list receipts");
    assert_eq!(
        receipts.len(),
        1,
        "{driver}: the rolled-back insert left no row"
    );

    let mut tally = ContractSaveTally {
        id: 0,
        hits: 1,
        snapshot: RowSnapshot::default(),
    };
    tally.save().await.expect("insert tally");
    let id = tally.id;
    recorder.take();
    let outcome = Orm::transaction(move |_| {
        Box::pin(async move {
            tally.hits = 2;
            tally.save().await?;
            Err::<(), Error>(Error::Validation("roll the transaction back".into()))
        })
    })
    .await;
    assert!(
        matches!(outcome, Err(Error::Validation(_))),
        "{driver}: {outcome:?}"
    );
    expected.check(
        &recorder.take(),
        1,
        &expected.update_returning("contract_save_tallies", "hits = ?"),
        "save inside Orm::transaction",
    );
    let stored = ContractSaveTally::find(id)
        .await
        .expect("load tally")
        .expect("stored tally");
    assert_eq!(
        stored.hits, 1,
        "{driver}: the outer rollback discarded the save"
    );
    recorder.take();

    // A query timeout bounds each statement of the save.
    let mut timed = stored;
    timed.hits = 3;
    Orm::set_query_timeout(30);
    let saved = timed.save().await;
    Orm::set_query_timeout(0);
    saved.expect("update under a query timeout");
    expected.check(
        &recorder.take(),
        1,
        &expected.update_returning("contract_save_tallies", "hits = ?"),
        "tracked update under a query timeout",
    );
}
