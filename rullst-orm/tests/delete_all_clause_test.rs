//! `delete_all()` renders only WHERE predicates, so clauses that would bound
//! or reshape the selection fail instead of widening the delete.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{Error, FromRow, Orm, RullstValue};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "bounded_logs")]
struct BoundedLog {
    id: i32,
    status: String,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "bounded_notes")]
struct BoundedNote {
    id: i32,
    body: String,
    deleted_at: Option<String>,
}

async fn remaining_logs() -> i64 {
    BoundedLog::unscoped().count().await.expect("count logs")
}

fn assert_rejected(result: Result<u64, Error>, clause: &str) {
    match result {
        Err(Error::Validation(message)) => {
            assert!(message.contains(clause), "{clause}: {message}");
            assert!(
                message.contains("delete_all() does not support"),
                "{message}"
            );
        }
        other => panic!("delete_all() with {clause} must fail closed: {other:?}"),
    }
}

#[tokio::test]
async fn bounded_or_joined_bulk_deletes_fail_instead_of_deleting_everything() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let pool = Orm::pool().expect("pool");
    for statement in [
        "CREATE TABLE bounded_logs (id INTEGER PRIMARY KEY, status TEXT NOT NULL)",
        "CREATE TABLE bounded_notes (id INTEGER PRIMARY KEY, body TEXT NOT NULL, deleted_at TEXT)",
        "CREATE TABLE bounded_teams (id INTEGER PRIMARY KEY, archived INTEGER NOT NULL)",
        "INSERT INTO bounded_logs (id, status) VALUES (1, 'stale'), (2, 'stale'), (3, 'stale'), (4, 'live'), (5, 'live')",
        "INSERT INTO bounded_notes (id, body) VALUES (1, 'a'), (2, 'b'), (3, 'c')",
    ] {
        rullst_orm::_sqlx::query(statement)
            .execute(pool)
            .await
            .expect("schema/seed statement");
    }

    assert_rejected(
        BoundedLog::query()
            .order_by("id")
            .limit(2)
            .delete_all()
            .await,
        "limit()",
    );
    assert_rejected(
        BoundedLog::query()
            .where_eq("status", "stale")
            .limit(1)
            .delete_all()
            .await,
        "limit()",
    );
    assert_rejected(BoundedLog::query().offset(1).delete_all().await, "offset()");
    assert_rejected(
        BoundedLog::query().order_by_desc("id").delete_all().await,
        "order_by()",
    );
    assert_rejected(
        BoundedLog::query()
            .join("bounded_teams", "bounded_teams.id", "=", "bounded_logs.id")
            .where_eq("bounded_teams.archived", 1)
            .delete_all()
            .await,
        "joins",
    );
    assert_rejected(
        BoundedLog::query().group_by("status").delete_all().await,
        "group_by()",
    );
    assert_rejected(
        BoundedLog::query()
            .with_raw(
                "stale_ids",
                "SELECT id FROM bounded_logs WHERE status = 'stale'",
            )
            .where_raw(
                "id IN (SELECT id FROM stale_ids)",
                Vec::<RullstValue>::new(),
            )
            .delete_all()
            .await,
        "CTEs",
    );
    // A directly assigned bound is recognised as well.
    let mut direct = BoundedLog::query();
    direct.limit = Some(1);
    assert_rejected(direct.delete_all().await, "limit()");
    assert_eq!(remaining_logs().await, 5);

    // Soft deletes use the same guard.
    assert_rejected(BoundedNote::query().limit(1).delete_all().await, "limit()");
    assert_eq!(BoundedNote::query().count().await.expect("live notes"), 3);

    // Filtered bulk deletes, including an explicit opt-out of the cap, still run.
    let deleted = BoundedLog::query()
        .where_eq("status", "stale")
        .delete_all()
        .await
        .expect("filtered bulk delete");
    assert_eq!(deleted, 3);
    let deleted = BoundedLog::query()
        .limit(1)
        .unsafe_unlimited()
        .delete_all()
        .await
        .expect("uncapped bulk delete");
    assert_eq!(deleted, 2);
    assert_eq!(remaining_logs().await, 0);
    let trashed = BoundedNote::query()
        .where_eq("body", "a")
        .delete_all()
        .await
        .expect("filtered soft delete");
    assert_eq!(trashed, 1);
    assert_eq!(BoundedNote::query().count().await.expect("live notes"), 2);
}
