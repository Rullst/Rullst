//! Raw CTE and select fragments cannot let `bind()` values displace the
//! mandatory tenant binding.
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

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "raw_notes")]
struct RawNote {
    id: i32,
    order_id: i32,
    body: String,
}

async fn setup() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let pool = Orm::pool().expect("pool");
    for statement in [
        "CREATE TABLE raw_orders (id INTEGER PRIMARY KEY, organization_id TEXT NOT NULL, total INTEGER NOT NULL)",
        "CREATE TABLE raw_notes (id INTEGER PRIMARY KEY, order_id INTEGER NOT NULL, body TEXT NOT NULL)",
        "INSERT INTO raw_orders (id, organization_id, total) VALUES (1, 'acme', 10), (2, 'globex', 20), (3, 'acme', 30)",
        "INSERT INTO raw_notes (id, order_id, body) VALUES (1, 1, 'hit'), (2, 2, 'hit'), (3, 3, 'miss')",
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
async fn raw_fragments_cannot_displace_scope_bindings() {
    setup().await;
    with_tenant("acme", scoped_assertions()).await;
    unscoped_assertions().await;
}

// Builders capture the tenant when `query()` runs, so they are built here.
async fn scoped_assertions() {
    // Before FROM, `bind()` values used to take the tenant binding's place:
    // the CTE received "acme" and `organization_id = ?` the caller's value.
    let displaced = RawOrder::query()
        .with_raw("hits", "SELECT order_id FROM raw_notes WHERE body = ?")
        .bind("globex")
        .where_raw("id IN (SELECT order_id FROM hits)", no_bindings())
        .get()
        .await;
    assert!(matches!(
        displaced,
        Err(Error::Validation(message)) if message.contains("with_raw() SQL contains bind markers")
    ));
    for rejected in [
        RawOrder::query().with_recursive_raw("ids", "SELECT ? AS n"),
        RawOrder::query().select_raw("id, organization_id, total + $1 AS total"),
    ] {
        assert!(matches!(
            rejected.get().await,
            Err(Error::Validation(message)) if message.contains("bind markers")
        ));
    }

    let literal = RawOrder::query()
        .with_raw("hits", "SELECT order_id FROM raw_notes WHERE body = 'hit?'")
        .where_raw("id IN (SELECT order_id FROM hits)", no_bindings())
        .get()
        .await
        .expect("a quoted question mark is not a bind marker");
    assert!(literal.is_empty());

    let constant = RawOrder::query()
        .with_raw("hits", "SELECT order_id FROM raw_notes WHERE body = 'hit'")
        .where_raw("id IN (SELECT order_id FROM hits)", no_bindings())
        .get()
        .await
        .expect("a raw CTE without markers keeps working under tenant scope");
    assert_eq!(
        constant.iter().map(|order| order.id).collect::<Vec<_>>(),
        [1]
    );
}

async fn unscoped_assertions() {
    // Without earlier bindings, `bind()` still fills a raw CTE marker first.
    let notes = RawNote::query()
        .with_raw("hits", "SELECT id FROM raw_orders WHERE total > ?")
        .bind(15)
        .where_raw("order_id IN (SELECT id FROM hits)", no_bindings())
        .order_by("id")
        .get()
        .await
        .expect("an unscoped raw CTE binds in textual order");
    assert_eq!(notes.iter().map(|note| note.id).collect::<Vec<_>>(), [2, 3]);

    // A WHERE value bound earlier would be consumed by the CTE marker.
    let displaced = RawNote::query()
        .where_eq("body", "hit")
        .with_raw("hits", "SELECT id FROM raw_orders WHERE total > ?")
        .bind(15)
        .get()
        .await;
    assert!(matches!(
        displaced,
        Err(Error::Validation(message)) if message.contains("bind markers")
    ));
}
