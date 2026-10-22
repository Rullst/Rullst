//! `#[orm(db_default)]` declares a column the database manages, such as the
//! generated `users.is_admin` and `users.session_epoch`: the ORM reads and
//! filters it but never writes it. `#[orm(skip)]` fields are not columns.
//! The SQLx `Any` pool decodes no SQLite boolean, so `is_admin` is an integer
//! flag here; generated projects use a strict driver and `bool`.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{FromRow, Orm, RowSnapshot};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "members")]
struct Member {
    id: i32,
    name: String,
    #[orm(db_default)]
    is_admin: i64,
    #[orm(db_default)]
    session_epoch: i64,
}

/// The same table through a model with dirty tracking.
#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "members")]
struct TrackedMember {
    id: i32,
    name: String,
    #[orm(db_default)]
    is_admin: i64,
    #[orm(db_default)]
    session_epoch: i64,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

async fn execute(sql: &str) {
    rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.to_string()))
        .execute(Orm::pool().expect("pool"))
        .await
        .unwrap_or_else(|error| panic!("{sql}: {error}"));
}

async fn stored_flags(id: i32) -> (i64, i64) {
    rullst_orm::_sqlx::query_as("SELECT is_admin, session_epoch FROM members WHERE id = ?")
        .bind(id)
        .fetch_one(Orm::pool().expect("pool"))
        .await
        .expect("read flags")
}

#[tokio::test]
async fn database_managed_columns_are_read_and_filtered_but_never_written() {
    Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .expect("initialize SQLite");
    execute(
        "CREATE TABLE members (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            is_admin INTEGER NOT NULL DEFAULT 0,
            session_epoch BIGINT NOT NULL DEFAULT 0
        )",
    )
    .await;

    // INSERT leaves both columns to their database defaults.
    let mut admin = Member {
        id: 0,
        name: "Ada".into(),
        is_admin: 1,
        session_epoch: 9,
    };
    admin.save().await.expect("insert admin");
    let mut member = Member {
        id: 0,
        name: "Grace".into(),
        is_admin: 0,
        session_epoch: 0,
    };
    member.save().await.expect("insert member");
    assert_eq!(stored_flags(admin.id).await, (0, 0));

    // Granted and bumped outside the ORM, then filtered through the builder.
    execute(&format!(
        "UPDATE members SET is_admin = 1, session_epoch = 3 WHERE id = {}",
        admin.id
    ))
    .await;
    let admins = Member::query()
        .where_eq("is_admin", 1_i64)
        .get()
        .await
        .expect("filter on is_admin");
    assert_eq!(admins.len(), 1);
    assert_eq!(admins[0].id, admin.id);
    assert_eq!(admins[0].is_admin, 1);
    assert_eq!(admins[0].session_epoch, 3);
    let current = Member::query()
        .where_eq("session_epoch", 3_i64)
        .order_by("session_epoch")
        .first()
        .await
        .expect("filter on session_epoch")
        .expect("bumped member");
    assert_eq!(current.id, admin.id);

    // A full-row save of a stale handle keeps the stored values.
    admin.name = "Ada Lovelace".into();
    admin.is_admin = 0;
    admin.session_epoch = 0;
    admin.save().await.expect("full-row save");
    assert_eq!(stored_flags(admin.id).await, (1, 3));

    // A tracked save refreshes them from the stored row.
    let mut tracked = TrackedMember::find(member.id)
        .await
        .expect("find tracked")
        .expect("tracked member");
    execute(&format!(
        "UPDATE members SET session_epoch = 5 WHERE id = {}",
        member.id
    ))
    .await;
    tracked.name = "Grace Hopper".into();
    tracked.is_admin = 1;
    assert_eq!(tracked.dirty_columns(), ["name"]);
    tracked.save().await.expect("tracked save");
    assert_eq!(stored_flags(member.id).await, (0, 5));
    assert_eq!(tracked.is_admin, 0);
    assert_eq!(tracked.session_epoch, 5);
}
