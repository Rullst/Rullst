#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::{FromRow, Orm};

#[derive(Clone, Debug, FromRow, Orm)]
#[orm(table = "binding_records")]
struct BindingRecord {
    id: i32,
    name: String,
}

#[tokio::test]
async fn bindings_follow_sql_clause_order_instead_of_builder_call_order() {
    Orm::init_with_options(
        "sqlite:file:binding_order_test.db?mode=memory&cache=shared",
        5,
        30,
    )
    .await
    .expect("initialize binding-order database");
    let pool = Orm::pool().expect("binding-order pool should exist");
    sqlx::query(
        "CREATE TABLE binding_records (\
            id INTEGER PRIMARY KEY AUTOINCREMENT,\
            name TEXT NOT NULL\
        )",
    )
    .execute(pool)
    .await
    .expect("create records table");
    sqlx::query(
        "CREATE TABLE binding_tags (\
            record_id INTEGER NOT NULL,\
            label TEXT NOT NULL\
        )",
    )
    .execute(pool)
    .await
    .expect("create tags table");
    sqlx::query("INSERT INTO binding_records (name) VALUES (?), (?)")
        .bind("alpha")
        .bind("beta")
        .execute(pool)
        .await
        .expect("seed records");
    sqlx::query("INSERT INTO binding_tags (record_id, label) VALUES (?, ?), (?, ?)")
        .bind(1_i32)
        .bind("match")
        .bind(2_i32)
        .bind("other")
        .execute(pool)
        .await
        .expect("seed tags");

    // The builder calls WHERE, JOIN and CTE in the reverse of their final SQL
    // positions. Distinct values make a binding-order regression observable.
    let query = BindingRecord::query()
        .where_eq("binding_records.name", "alpha")
        .join_constrained("binding_tags", |join| {
            join.on("binding_records.id", "=", "binding_tags.record_id")
                .on_eq("binding_tags.label", "match")
        })
        .with_cte(
            "unused_beta",
            BindingRecord::query().where_eq("name", "beta"),
        );

    let rows = query.get().await.expect("ordered bindings should query");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "alpha");
    assert_eq!(query.count().await.expect("ordered count bindings"), 1);
    let page = query
        .paginate(1, 10)
        .await
        .expect("ordered pagination bindings");
    assert_eq!(page.total, 1);
    assert_eq!(page.data.len(), 1);
}
