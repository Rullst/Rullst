//! `i64` values and `None` bind on every driver: filters on a `BIGINT`
//! column beyond `i32`, `Option` filters and a raw `NULL` assignment.

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_wide_values")]
struct ContractWideValue {
    id: i32,
    views: i64,
    note: Option<String>,
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_wide_values", |table: &mut Blueprint| {
        table.id();
        table.big_integer("views").not_null();
        table.string("note");
    })
    .await
    .expect("create wide-value contract table");

    let mut wide = ContractWideValue {
        id: 0,
        views: 5_000_000_000,
        note: Some("wide".to_string()),
    };
    wide.save().await.expect("insert wide row");
    let mut narrow = ContractWideValue {
        id: 0,
        views: 7,
        note: Some("narrow".to_string()),
    };
    narrow.save().await.expect("insert narrow row");

    let found = ContractWideValue::query()
        .where_eq("views", 5_000_000_000_i64)
        .get()
        .await
        .unwrap_or_else(|error| panic!("{driver} i64 filter: {error}"));
    assert_eq!(
        found.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![wide.id],
        "{driver} i64 filter"
    );
    let found = ContractWideValue::query()
        .where_eq("views", Some(7_i64))
        .get()
        .await
        .unwrap_or_else(|error| panic!("{driver} Option filter: {error}"));
    assert_eq!(
        found.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![narrow.id],
        "{driver} Option filter"
    );
    let counted = ContractWideValue::query()
        .where_in("views", vec![7_i64, 5_000_000_000])
        .count()
        .await
        .unwrap_or_else(|error| panic!("{driver} i64 where_in: {error}"));
    assert_eq!(counted, 2, "{driver} i64 where_in");

    let clear = if driver == "postgres" {
        "UPDATE contract_wide_values SET note = $1 WHERE views = $2"
    } else {
        "UPDATE contract_wide_values SET note = ? WHERE views = ?"
    };
    let cleared = Orm::raw(clear)
        .bind(None::<String>)
        .bind(5_000_000_000_i64)
        .execute()
        .await
        .unwrap_or_else(|error| panic!("{driver} raw NULL: {error}"));
    assert_eq!(cleared, 1, "{driver} raw NULL matched the wide row");
    let stored = ContractWideValue::find(wide.id)
        .await
        .expect("load wide row")
        .expect("stored wide row");
    assert_eq!(
        (stored.views, stored.note),
        (5_000_000_000, None),
        "{driver} the NULL was stored"
    );

    Schema::drop_if_exists("contract_wide_values")
        .await
        .expect("drop wide-value contract table");
}
