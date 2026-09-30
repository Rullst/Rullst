//! Raw CTE and select fragments take their own bindings at their textual
//! position, so the mandatory tenant binding always reaches its predicate.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{Error, FromRow, Orm, RullstValue, with_tenant};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "raw_orders", tenant_column = "organization_id")]
struct RawOrder {
    id: i32,
    organization_id: String,
    total: i32,
}

async fn setup() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let pool = Orm::pool().expect("pool");
    for statement in [
        "CREATE TABLE raw_orders (id INTEGER PRIMARY KEY, organization_id TEXT NOT NULL, total INTEGER NOT NULL)",
        "CREATE TABLE raw_notes (order_id INTEGER NOT NULL, body TEXT NOT NULL)",
        "INSERT INTO raw_orders (id, organization_id, total) VALUES (1, 'acme', 10), (2, 'globex', 20), (3, 'acme', 30)",
        "INSERT INTO raw_notes (order_id, body) VALUES (1, 'hit'), (2, 'hit'), (3, 'miss')",
    ] {
        rullst_orm::_sqlx::query(statement)
            .execute(pool)
            .await
            .expect("schema/seed statement");
    }
}

fn no_bindings() -> Vec<RullstValue> {
    Vec::new()
}

#[tokio::test]
async fn raw_fragments_bind_in_textual_order_under_tenant_scope() {
    setup().await;
    with_tenant("acme", scoped_assertions()).await;
}

// Builders capture the tenant when `query()` runs, so they are built here.
async fn scoped_assertions() {
    let hits = RawOrder::query()
        .with_raw_bindings(
            "hits",
            "SELECT order_id FROM raw_notes WHERE body = ?",
            vec!["hit"],
        )
        .where_raw("id IN (SELECT order_id FROM hits)", no_bindings())
        .get()
        .await
        .expect("raw CTE with its own binding");
    assert_eq!(hits.iter().map(|order| order.id).collect::<Vec<_>>(), [1]);

    let recursive = RawOrder::query()
        .with_recursive_raw_bindings(
            "ids",
            "SELECT ? AS n UNION ALL SELECT n + 1 FROM ids WHERE n < ?",
            vec![1, 3],
        )
        .where_raw("id IN (SELECT n FROM ids)", no_bindings())
        .order_by("id")
        .get()
        .await
        .expect("recursive raw CTE with its own bindings");
    assert_eq!(
        recursive.iter().map(|order| order.id).collect::<Vec<_>>(),
        [1, 3]
    );

    let adjusted = RawOrder::query()
        .select_raw_bindings("id, organization_id, total + ? AS total", vec![5])
        .where_eq("total", 30)
        .get()
        .await
        .expect("raw select with its own binding");
    assert_eq!(adjusted.len(), 1);
    assert_eq!(adjusted[0].total, 35);

    // Pluck and count replace the select list, so its binding is left out.
    let bound = || {
        RawOrder::query()
            .select_raw_bindings("id, organization_id, total + ? AS total", vec![5])
            .order_by("id")
    };
    assert_eq!(bound().pluck_i32("id").await.expect("pluck"), [1, 3]);
    assert_eq!(bound().count().await.expect("count"), 2);

    for mismatched in [
        RawOrder::query().select_raw_bindings("total + ?", no_bindings()),
        RawOrder::query().with_raw_bindings("hits", "SELECT 1", vec![1]),
    ] {
        assert!(matches!(
            mismatched.get().await,
            Err(Error::Validation(message)) if message.contains("bind marker")
        ));
    }
}
