//! Bind markers of `delete_all()` and of the `cascade_soft_delete` statement
//! it issues, per driver. MySQL/MariaDB and SQLite must keep `?`; PostgreSQL
//! must receive `$1..$n` in binding order. The live matrices execute the same
//! statements through `driver_contract`.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{FromRow, Orm, with_tenant};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "bulk_items")]
struct BulkItem {
    id: i32,
    score: i32,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "tenant_bulk_items", tenant_column = "tenant_id")]
struct TenantBulkItem {
    id: i32,
    tenant_id: String,
    score: i32,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "bulk_posts")]
struct BulkPost {
    id: i32,
    #[sqlx(default, skip)]
    #[orm(
        has_many = "BulkComment",
        foreign_key = "bulk_post_id",
        cascade_soft_delete
    )]
    comments: Option<Vec<BulkComment>>,
    deleted_at: Option<String>,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "bulk_comments")]
struct BulkComment {
    id: i32,
    bulk_post_id: i32,
    deleted_at: Option<String>,
}

const NON_POSTGRES: [&str; 2] = ["mysql", "sqlite"];

#[test]
fn filtered_delete_all_keeps_question_marks_off_postgres() {
    let query = BulkItem::query().where_eq("score", 1).or_where("score", 2);
    assert_eq!(query.bindings.len(), 2);
    for driver in NON_POSTGRES {
        assert_eq!(
            query.__rullst_delete_all_sql(driver),
            "DELETE FROM bulk_items WHERE ((score = ?) OR (score = ?))",
            "{driver}"
        );
    }
    assert_eq!(
        query.__rullst_delete_all_sql("postgres"),
        "DELETE FROM bulk_items WHERE ((score = $1) OR (score = $2))"
    );
    assert_eq!(
        BulkItem::query().__rullst_delete_all_sql("mysql"),
        "DELETE FROM bulk_items"
    );
}

#[tokio::test]
async fn tenant_scoped_delete_all_numbers_the_scope_binding_first() {
    let query = with_tenant("acme", async {
        TenantBulkItem::query().where_eq("score", 1)
    })
    .await;
    assert!(query.errors.is_empty(), "{:?}", query.errors);
    for driver in NON_POSTGRES {
        assert_eq!(
            query.__rullst_delete_all_sql(driver),
            "DELETE FROM tenant_bulk_items WHERE (tenant_id = ?) AND (score = ?)",
            "{driver}"
        );
    }
    assert_eq!(
        query.__rullst_delete_all_sql("postgres"),
        "DELETE FROM tenant_bulk_items WHERE (tenant_id = $1) AND (score = $2)"
    );
}

#[test]
fn cascade_soft_delete_statement_keeps_question_marks_off_postgres() {
    // The generated cascade runs exactly this child builder through
    // delete_all_with_tx when a BulkPost is deleted.
    let cascade = BulkComment::query().where_eq("bulk_post_id", 7);
    for driver in NON_POSTGRES {
        assert_eq!(
            cascade.__rullst_delete_all_sql(driver),
            "UPDATE bulk_comments SET deleted_at = CURRENT_TIMESTAMP \
             WHERE (bulk_post_id = ?) AND deleted_at IS NULL",
            "{driver}"
        );
    }
    assert_eq!(
        cascade.__rullst_delete_all_sql("postgres"),
        "UPDATE bulk_comments SET deleted_at = CURRENT_TIMESTAMP \
         WHERE (bulk_post_id = $1) AND deleted_at IS NULL"
    );
}

#[tokio::test]
async fn delete_all_and_cascade_execute_with_the_active_driver() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-delete-all-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    Orm::init(&format!("sqlite:{}?mode=rwc", database_path.display()))
        .await
        .expect("initialize SQLite delete_all database");
    let pool = Orm::pool().expect("SQLite pool");
    for statement in [
        "CREATE TABLE bulk_items (id INTEGER PRIMARY KEY, score INTEGER NOT NULL)",
        "INSERT INTO bulk_items (id, score) VALUES (1, 1), (2, 2), (3, 3)",
        "CREATE TABLE bulk_posts (id INTEGER PRIMARY KEY, deleted_at TEXT)",
        "CREATE TABLE bulk_comments (id INTEGER PRIMARY KEY, bulk_post_id INTEGER NOT NULL, deleted_at TEXT)",
        "INSERT INTO bulk_posts (id) VALUES (1), (2)",
        "INSERT INTO bulk_comments (id, bulk_post_id) VALUES (1, 1), (2, 1), (3, 2)",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .expect("seed delete_all fixture");
    }

    let deleted = BulkItem::query()
        .where_eq("score", 1)
        .or_where("score", 3)
        .delete_all()
        .await
        .expect("filtered delete_all");
    assert_eq!(deleted, 2);
    assert_eq!(BulkItem::query().pluck_i32("id").await.unwrap(), vec![2]);

    let post = BulkPost::find(1).await.unwrap().expect("post 1");
    post.delete().await.expect("cascade soft delete");
    assert_eq!(BulkComment::query().pluck_i32("id").await.unwrap(), vec![3]);
    assert_eq!(
        BulkComment::query().with_trashed().count().await.unwrap(),
        3
    );
    assert!(BulkPost::find(1).await.unwrap().is_none());

    pool.close().await;
    let _ = std::fs::remove_file(database_path);
}
