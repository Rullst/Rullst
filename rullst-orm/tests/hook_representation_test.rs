//! Rows re-saved by `update_partial()` and `restore_revision()` pass through
//! `after_fetch` first, so a non-idempotent `before_save` mutator is applied
//! exactly once, as for `find()` followed by `save()`.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::audit::{AuditContext, with_audit_context};
use rullst_orm::{Error, FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(
    table = "hooked_lessons",
    auditable,
    before_save = "encode_notes",
    after_fetch = "decode_notes"
)]
struct HookedLesson {
    id: i32,
    title: String,
    notes: String,
}

impl HookedLesson {
    /// Mutator: stores the notes with a marker prefix.
    async fn encode_notes(&mut self) -> Result<(), Error> {
        self.notes = format!("enc:{}", self.notes);
        Ok(())
    }

    /// Accessor: removes the marker prefix of stored notes.
    async fn decode_notes(&mut self) -> Result<(), Error> {
        if let Some(plain) = self.notes.strip_prefix("enc:") {
            self.notes = plain.to_string();
        }
        Ok(())
    }
}

async fn stored(id: i32) -> (String, String) {
    sqlx::query_as("SELECT title, notes FROM hooked_lessons WHERE id = ?")
        .bind(id)
        .fetch_one(Orm::pool().expect("pool"))
        .await
        .expect("read stored lesson")
}

async fn latest_update_audit_id(id: i32) -> i32 {
    let row: (i32,) = sqlx::query_as(
        "SELECT id FROM rullst_audits WHERE model_type = 'hooked_lessons' AND model_id = ? AND event = 'updated' ORDER BY id DESC LIMIT 1",
    )
    .bind(id)
    .fetch_one(Orm::pool().expect("pool"))
    .await
    .expect("read latest update audit");
    row.0
}

#[tokio::test]
async fn resaved_rows_use_the_after_fetch_representation() {
    Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .expect("initialize SQLite");
    sqlx::query(
        "CREATE TABLE hooked_lessons (id INTEGER PRIMARY KEY, title TEXT NOT NULL, notes TEXT NOT NULL)",
    )
    .execute(Orm::pool().expect("pool"))
    .await
    .expect("create lessons");
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create audit table");
    let context = AuditContext::system("hook-representation").expect("audit context");
    with_audit_context(context, scenario()).await;
}

async fn scenario() {
    let mut lesson = HookedLesson {
        id: 0,
        title: "draft".into(),
        notes: "hello".into(),
    };
    lesson.save().await.expect("create lesson");
    assert_eq!(stored(lesson.id).await.1, "enc:hello");

    // A normal read-modify-save encodes once; restoring that revision must too.
    let mut loaded = HookedLesson::find(lesson.id)
        .await
        .expect("find")
        .expect("lesson exists");
    assert_eq!(loaded.notes, "hello");
    loaded.title = "published".into();
    loaded.save().await.expect("update title");
    assert_eq!(
        stored(lesson.id).await,
        ("published".into(), "enc:hello".into())
    );
    let audit_id = latest_update_audit_id(lesson.id).await;
    loaded
        .restore_revision(audit_id, "undo publication")
        .await
        .expect("restore revision");
    assert_eq!(
        stored(lesson.id).await,
        ("draft".into(), "enc:hello".into())
    );

    // A partial update of another column leaves the notes encoded once.
    let mut current = HookedLesson::find(lesson.id)
        .await
        .expect("find")
        .expect("lesson exists");
    current
        .update_partial()
        .title("revised".into())
        .save()
        .await
        .expect("partial update");
    assert_eq!(
        stored(lesson.id).await,
        ("revised".into(), "enc:hello".into())
    );
    let reread = HookedLesson::find(lesson.id)
        .await
        .expect("find")
        .expect("lesson exists");
    assert_eq!(reread.notes, "hello");
    assert_eq!(current.title, "revised");
}
