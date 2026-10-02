//! A model without soft deletes has no trashed rows. `only_trashed()` fails
//! with `Validation` instead of letting reads and `delete_all()` act on every
//! live row as if it were the trash.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{Error, FromRow, Orm};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "trashless_line_items")]
struct TrashlessLineItem {
    id: i32,
    sku: String,
}

fn assert_rejected<T: std::fmt::Debug>(result: Result<T, Error>, operation: &str) {
    match result {
        Err(Error::Validation(message)) => {
            assert!(message.contains("only_trashed()"), "{operation}: {message}");
            assert!(message.contains("soft-delete"), "{operation}: {message}");
        }
        other => panic!("{operation} with only_trashed() must fail closed: {other:?}"),
    }
}

async fn live_rows() -> i64 {
    TrashlessLineItem::query()
        .count()
        .await
        .expect("count rows")
}

#[tokio::test]
async fn only_trashed_fails_closed_on_models_without_soft_deletes() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let pool = Orm::pool().expect("pool");
    for statement in [
        "CREATE TABLE trashless_line_items (id INTEGER PRIMARY KEY, sku TEXT NOT NULL)",
        "INSERT INTO trashless_line_items (id, sku) VALUES (1, 'a'), (2, 'b'), (3, 'c')",
    ] {
        rullst_orm::_sqlx::query(statement)
            .execute(pool)
            .await
            .expect("schema/seed statement");
    }

    let trash = || TrashlessLineItem::query().only_trashed();
    assert_rejected(trash().delete_all().await, "delete_all()");
    assert_rejected(trash().count().await, "count()");
    assert_rejected(trash().get().await, "get()");
    assert_rejected(trash().first().await, "first()");
    assert_rejected(trash().pluck_i32("id").await, "pluck_i32()");
    assert_eq!(live_rows().await, 3, "an empty-trash job deleted live rows");

    // A flag set directly on the public field cannot widen the statement:
    // no row of this model is trashed.
    let mut direct = TrashlessLineItem::query();
    direct.only_trashed = true;
    assert!(direct.to_sql().contains("1 = 0"), "{}", direct.to_sql());
    assert_eq!(direct.count().await.expect("direct flag count"), 0);
    assert_eq!(
        direct.delete_all().await.expect("direct flag delete_all"),
        0
    );
    assert_eq!(live_rows().await, 3);

    // `with_trashed()` cannot widen a model without trash and stays accepted.
    assert_eq!(
        TrashlessLineItem::query()
            .with_trashed()
            .count()
            .await
            .expect("with_trashed count"),
        3
    );
}
