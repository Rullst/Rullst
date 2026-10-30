//! `save()` writes the whole row, so the last writer wins; `update_partial()`
//! and one atomic `UPDATE ... SET xp = xp + ?` keep concurrent changes.

use rullst_orm::Orm;
use rullst_orm::schema::{Blueprint, Schema};

#[derive(Debug, Clone, Orm)]
#[orm(table = "contract_counters")]
struct ContractCounter {
    id: i32,
    label: String,
    xp: i32,
}

async fn stored(id: i32) -> ContractCounter {
    ContractCounter::find(id)
        .await
        .expect("load counter")
        .expect("stored counter")
}

/// Adds `points` with one statement, as the ORM guide documents. PostgreSQL
/// numbers its markers; SQLite and MySQL/MariaDB take `?`.
async fn award(id: i32, points: i32) -> u64 {
    let sql = if Orm::driver().expect("initialized driver") == "postgres" {
        "UPDATE contract_counters SET xp = xp + $1 WHERE id = $2"
    } else {
        "UPDATE contract_counters SET xp = xp + ? WHERE id = ?"
    };
    Orm::raw(sql)
        .bind(points)
        .bind(id)
        .execute()
        .await
        .expect("atomic increment")
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_counters", |table: &mut Blueprint| {
        table.id();
        table.string("label").not_null();
        table.integer("xp").not_null();
    })
    .await
    .expect("create counter contract table");

    let mut counter = ContractCounter {
        id: 0,
        label: "student".to_string(),
        xp: 0,
    };
    counter.save().await.expect("insert counter");
    let loaded_before_awards = counter.clone();

    let awards = futures::future::join_all((0..8).map(|_| award(counter.id, 5))).await;
    assert_eq!(
        awards,
        vec![1; 8],
        "{driver} every increment matched the row"
    );
    assert_eq!(
        stored(counter.id).await.xp,
        40,
        "{driver} no increment was lost"
    );

    // A full-row save() from a handle loaded before the awards writes its
    // stale counter back.
    let mut stale = loaded_before_awards;
    stale.label = "renamed by save".to_string();
    stale.save().await.expect("full-row save");
    let row = stored(counter.id).await;
    assert_eq!(
        (row.label.as_str(), row.xp),
        ("renamed by save", 0),
        "{driver} save() is last-writer-wins"
    );

    // update_partial() merges only its fields into the current row.
    assert_eq!(award(counter.id, 7).await, 1);
    counter
        .update_partial()
        .label("renamed".to_string())
        .save()
        .await
        .expect("partial update");
    let row = stored(counter.id).await;
    assert_eq!(
        (row.label.as_str(), row.xp),
        ("renamed", 7),
        "{driver} update_partial() keeps the concurrent increment"
    );
    assert_eq!(
        counter.xp, 7,
        "{driver} the handle is refreshed from the row"
    );

    Schema::drop_if_exists("contract_counters")
        .await
        .expect("drop counter contract table");
}
