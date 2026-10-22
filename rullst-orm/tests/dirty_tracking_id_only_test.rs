//! A tracked model with no writable column (what `make:model` generates before
//! fields are added) compiles without unreachable code and never writes.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![deny(unreachable_code)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{FromRow, Orm, RowSnapshot};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "markers")]
struct Marker {
    id: i32,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

#[tokio::test]
async fn saving_an_id_only_tracked_record_inserts_once_and_then_writes_nothing() {
    Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .expect("initialize SQLite");
    rullst_orm::_sqlx::query("CREATE TABLE markers (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .execute(Orm::pool().expect("pool"))
        .await
        .expect("create table");

    let mut marker = Marker {
        id: 0,
        snapshot: RowSnapshot::default(),
    };
    marker.save().await.expect("insert");
    assert_eq!(marker.id, 1);
    assert!(!marker.is_dirty());

    let mut loaded = Marker::find(1).await.expect("find").expect("stored row");
    assert!(!loaded.is_dirty());
    loaded.save().await.expect("save without changes");
    assert_eq!(loaded.id, 1);
    let count: (i64,) = rullst_orm::_sqlx::query_as("SELECT COUNT(*) FROM markers")
        .fetch_one(Orm::pool().expect("pool"))
        .await
        .expect("count");
    assert_eq!(count.0, 1);
}
