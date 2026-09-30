//! `save()` never writes the soft-delete marker, so a handle loaded before
//! `delete()` cannot undelete its row, and `update_partial()` cannot set it.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::audit::{AuditContext, with_audit_context};
use rullst_orm::{Error, FromRow, Orm};

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "sd_posts")]
struct SdPost {
    id: i32,
    title: String,
    deleted_at: Option<String>,
}

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "sd_audited_posts", auditable)]
struct SdAuditedPost {
    id: i32,
    title: String,
    deleted_at: Option<String>,
}

async fn trashed_title(table: &str, id: i32) -> (String, Option<String>) {
    let sql = format!("SELECT title, deleted_at FROM {table} WHERE id = ?");
    sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_one(Orm::pool().expect("pool"))
        .await
        .expect("read stored row")
}

#[tokio::test]
async fn stale_handles_cannot_undelete_through_save_or_partial_updates() {
    Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .expect("initialize SQLite");
    for statement in [
        "CREATE TABLE sd_posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL, deleted_at TEXT)",
        "CREATE TABLE sd_audited_posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL, deleted_at TEXT)",
    ] {
        sqlx::query(statement)
            .execute(Orm::pool().expect("pool"))
            .await
            .expect("create table");
    }
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create audit table");

    let mut post = SdPost {
        id: 0,
        title: "draft".into(),
        deleted_at: None,
    };
    post.save().await.expect("create post");
    let mut stale = post.clone();
    post.delete().await.expect("soft delete");

    post.title = "edited".into();
    post.save()
        .await
        .expect("editing a trashed row stays allowed");
    stale.title = "concurrent edit".into();
    stale
        .save()
        .await
        .expect("save a handle loaded before delete()");
    assert!(SdPost::find(post.id).await.expect("find").is_none());
    let (title, deleted_at) = trashed_title("sd_posts", post.id).await;
    assert_eq!(title, "concurrent edit");
    assert!(deleted_at.is_some(), "save() must not undelete the row");

    let mut trashed = SdPost::query()
        .with_trashed()
        .where_eq("id", post.id)
        .first()
        .await
        .expect("load trashed post")
        .expect("trashed post exists");
    let restore_attempt = trashed.update_partial().deleted_at(None).save().await;
    assert!(matches!(
        restore_attempt,
        Err(Error::Validation(message)) if message.contains("soft-delete column")
    ));
    assert!(trashed_title("sd_posts", post.id).await.1.is_some());

    let context = AuditContext::system("soft-delete-save").expect("audit context");
    with_audit_context(context, async {
        let mut audited = SdAuditedPost {
            id: 0,
            title: "draft".into(),
            deleted_at: None,
        };
        audited.save().await.expect("create audited post");
        let mut stale = audited.clone();
        audited.delete().await.expect("soft delete audited post");
        stale.title = "edited".into();
        stale.save().await.expect("save stale audited handle");
        assert!(stale.deleted_at.is_some(), "the handle takes the stored marker");
        assert!(trashed_title("sd_audited_posts", stale.id).await.1.is_some());
        let (new_values,): (String,) = sqlx::query_as(
            "SELECT new_values FROM rullst_audits WHERE model_type = 'sd_audited_posts' AND event = 'updated' ORDER BY id DESC LIMIT 1",
        )
        .fetch_one(Orm::pool().expect("pool"))
        .await
        .expect("read update audit");
        let new_values: serde_json::Value =
            serde_json::from_str(&new_values).expect("audit JSON");
        assert_ne!(
            new_values.get("deleted_at"),
            Some(&serde_json::Value::Null),
            "the audit must not record an undelete"
        );
    })
    .await;
}
