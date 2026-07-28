//! Demonstrates the configurable soft delete and the `#[orm(skip)]` /
//! `#[sqlx(skip)]` field attributes.
//!
//! Both features are inspired by the equivalent MyBatis-Plus knobs and
//! work on every driver supported by `rullst-orm` (MySQL, PostgreSQL,
//! SQLite). The model below uses a custom soft delete column named
//! `is_deleted` with an `i32` sentinel; the `secret` field is excluded
//! from generated SQL via `#[orm(skip)]`.

use rullst_orm::{FromRow, Orm};

// `is_deleted: 0` is the "not deleted" sentinel. The `delval`
// expression `1` is interpolated verbatim into the generated UPDATE
// statement, so it works on MySQL, PostgreSQL and SQLite alike.
// Replace `1` with `now()` / `UNIX_TIMESTAMP()` for timestamp-based
// multi-delete scenarios.
#[derive(Debug, Clone, Default, FromRow, rullst_orm::Orm)]
#[orm(
    table = "soft_delete_demo",
    soft_delete(field = "is_deleted", value = "0", delval = "1")
)]
pub struct SoftDeleteDemo {
    pub id: i32,
    pub name: String,
    pub is_deleted: i32,
    // `secret` is intentionally NOT persisted. The macro removes it
    // from generated INSERT / UPDATE column lists, the `*Column` enum
    // and the JSON serialiser. The field remains on the struct so user
    // code can still read/write it locally. We use `#[sqlx(skip)]`
    // (the alias recognised by both this ORM derive and the
    // `sqlx::FromRow` derive) so the database row mapping also skips
    // the field — there is no `secret` column in the schema.
    #[sqlx(skip)]
    pub secret: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::remove_file("test_soft_delete.db");
    std::fs::File::create("test_soft_delete.db").unwrap();
    Orm::init("sqlite:test_soft_delete.db").await?;
    let pool = Orm::pool();

    // Set up a table that mirrors the `is_deleted` column declared on
    // the model. SQLite (and MySQL/PostgreSQL) will accept the same
    // DDL.
    sqlx::query(
        "CREATE TABLE soft_delete_demo (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, is_deleted INTEGER NOT NULL DEFAULT 0)",
    )
    .execute(pool)
    .await?;

    // ── INSERT ───────────────────────────────────────────────────────────
    let mut row = SoftDeleteDemo {
        id: 0,
        name: "Alice".to_string(),
        is_deleted: 0,
        secret: "shhh, this is not persisted".to_string(),
    };
    row.save().await?;
    println!("inserted id={} name={}", row.id, row.name);

    // ── READ (only non-deleted) ──────────────────────────────────────────
    let visible = SoftDeleteDemo::query().get().await?;
    println!("visible rows: {}", visible.len());
    assert_eq!(visible.len(), 1, "fresh row should be visible");

    // The locally maintained `secret` value is still available even
    // though it is not persisted.
    println!("local secret after insert: {}", row.secret);

    // ── SOFT DELETE ──────────────────────────────────────────────────────
    // Generates: UPDATE soft_delete_demo SET is_deleted = 1 WHERE id = ?
    row.delete().await?;
    let visible_after = SoftDeleteDemo::query().get().await?;
    println!(
        "visible rows after soft delete: {} (expected 0)",
        visible_after.len()
    );
    assert_eq!(visible_after.len(), 0, "soft delete must hide the row");

    // `.with_trashed()` keeps soft-deleted rows visible.
    let all = SoftDeleteDemo::query().with_trashed().get().await?;
    println!("rows including trashed: {}", all.len());
    assert_eq!(all.len(), 1, "with_trashed should still see the row");

    // `.only_trashed()` returns just the deleted rows.
    let trashed = SoftDeleteDemo::query().only_trashed().get().await?;
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].is_deleted, 1);

    // ── RESTORE ──────────────────────────────────────────────────────────
    // Generates: UPDATE soft_delete_demo SET is_deleted = 0 WHERE id = ?
    row.restore().await?;
    let visible_again = SoftDeleteDemo::query().get().await?;
    println!("visible rows after restore: {}", visible_again.len());
    assert_eq!(
        visible_again.len(),
        1,
        "restore should make the row visible again"
    );

    // ── FORCE DELETE ─────────────────────────────────────────────────────
    row.force_delete().await?;
    let gone = SoftDeleteDemo::query().with_trashed().get().await?;
    println!("rows after force_delete: {} (expected 0)", gone.len());
    assert_eq!(gone.len(), 0, "force_delete removes the row entirely");

    // Cleanup
    let _ = std::fs::remove_file("test_soft_delete.db");
    Ok(())
}
