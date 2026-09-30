//! Offset reads without a row cap must stay valid SQL on every driver:
//! SQLite and MySQL/MariaDB accept `OFFSET` only after `LIMIT`.

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_offset_rows")]
struct ContractOffsetRow {
    id: i32,
    name: String,
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_offset_rows", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
    })
    .await
    .expect("create offset contract table");
    for name in ["e", "d", "c", "b", "a"] {
        let mut row = ContractOffsetRow {
            id: 0,
            name: name.to_string(),
        };
        row.save().await.expect("insert offset contract row");
    }

    let uncapped = ContractOffsetRow::query()
        .unsafe_unlimited()
        .order_by("id")
        .offset(2);
    let rows = uncapped
        .get()
        .await
        .unwrap_or_else(|error| panic!("{driver} uncapped offset get: {error}"));
    assert_eq!(
        rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
        ["c", "b", "a"],
        "{driver}"
    );
    let names = uncapped
        .pluck_string("name")
        .await
        .unwrap_or_else(|error| panic!("{driver} uncapped offset pluck: {error}"));
    assert_eq!(names, ["c", "b", "a"], "{driver}");
}
