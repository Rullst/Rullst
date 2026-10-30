//! A model with a `RowSnapshot` field: `save()` writes only the columns whose
//! value changed. SQLite `UPDATE OF <column>` triggers record exactly which
//! columns each UPDATE statement assigned.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{Error, Json, Orm, RowSnapshot};
use serde_json::json;
use std::time::Duration;

const ENCRYPTION_KEY: &str = "0123456789abcdef0123456789abcdef";

#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "tracked_posts")]
struct TrackedPost {
    id: i32,
    title: String,
    body: String,
    #[orm(encrypted)]
    secret: String,
    #[orm(encrypted)]
    note: Option<String>,
    meta: Json<serde_json::Value>,
    #[orm(db_default)]
    revision: i32,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

const TRACKED_COLUMNS: [&str; 5] = ["title", "body", "secret", "note", "meta"];

async fn execute(sql: &str) {
    rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.to_string()))
        .execute(Orm::pool().expect("pool"))
        .await
        .unwrap_or_else(|error| panic!("{sql}: {error}"));
}

/// Columns assigned by UPDATE statements since the last call, sorted (SQLite
/// does not order the triggers of one statement).
async fn take_writes() -> Vec<String> {
    let pool = Orm::pool().expect("pool");
    let rows: Vec<(String,)> =
        rullst_orm::_sqlx::query_as("SELECT col FROM column_writes ORDER BY id")
            .fetch_all(pool)
            .await
            .expect("read column writes");
    rullst_orm::_sqlx::query("DELETE FROM column_writes")
        .execute(pool)
        .await
        .expect("clear column writes");
    let mut columns: Vec<String> = rows.into_iter().map(|(column,)| column).collect();
    columns.sort();
    columns
}

/// Every tracked column, sorted like `take_writes()`.
const ALL_WRITES: [&str; 5] = ["body", "meta", "note", "secret", "title"];

async fn stored_ciphertext(id: i32) -> String {
    let row: (String,) =
        rullst_orm::_sqlx::query_as("SELECT secret FROM tracked_posts WHERE id = ?")
            .bind(id)
            .fetch_one(Orm::pool().expect("pool"))
            .await
            .expect("read ciphertext");
    row.0
}

async fn load(id: i32) -> TrackedPost {
    TrackedPost::find(id)
        .await
        .expect("find post")
        .expect("stored post")
}

async fn create_schema() {
    execute(
        "CREATE TABLE tracked_posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            secret TEXT NOT NULL,
            note TEXT,
            meta TEXT NOT NULL,
            revision INTEGER NOT NULL DEFAULT 0
        )",
    )
    .await;
    execute("CREATE TABLE column_writes (id INTEGER PRIMARY KEY AUTOINCREMENT, col TEXT NOT NULL)")
        .await;
    for column in TRACKED_COLUMNS.iter().chain(&["revision"]) {
        execute(&format!(
            "CREATE TRIGGER write_{column} AFTER UPDATE OF {column} ON tracked_posts \
             BEGIN INSERT INTO column_writes (col) VALUES ('{column}'); END"
        ))
        .await;
    }
}

#[tokio::test]
async fn save_writes_only_the_changed_columns() {
    // This test is the only one in its process, so no other thread reads the
    // process-wide key while it is set.
    unsafe {
        std::env::set_var("RULLST_ENCRYPTION_KEY", ENCRYPTION_KEY);
        std::env::remove_var("RULLST_ENCRYPTION_KEY_ID");
        std::env::remove_var("RULLST_ENCRYPTION_KEYRING");
    }
    Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .expect("initialize SQLite");
    create_schema().await;

    // Insert: unchanged, every column but the database default.
    let mut post = TrackedPost {
        id: 0,
        title: "draft".into(),
        body: "first body".into(),
        secret: "s3cret".into(),
        note: Some("note".into()),
        meta: Json(json!({ "b": 1, "a": { "y": 2, "x": 1 } })),
        revision: 41,
        snapshot: RowSnapshot::default(),
    };
    assert!(!post.snapshot.is_tracked());
    assert!(post.is_dirty());
    assert_eq!(post.dirty_columns(), TRACKED_COLUMNS);
    post.save().await.expect("insert post");
    let id = post.id;
    assert!(id > 0);
    assert!(post.snapshot.is_tracked());
    assert!(!post.is_dirty());
    assert!(post.dirty_columns().is_empty());
    assert_eq!(
        load(id).await.revision,
        0,
        "INSERT leaves db_default columns out"
    );
    assert!(take_writes().await.is_empty());

    // A loaded, unchanged record saves without touching the database: the
    // only pooled connection stays checked out while it saves.
    let mut loaded = load(id).await;
    assert!(loaded.snapshot.is_tracked());
    assert!(!loaded.is_dirty());
    let held = Orm::pool()
        .expect("pool")
        .acquire()
        .await
        .expect("hold connection");
    tokio::time::timeout(Duration::from_secs(5), loaded.save())
        .await
        .expect("an unchanged save needs no connection")
        .expect("save unchanged post");
    drop(held);
    assert!(take_writes().await.is_empty());

    // One changed field updates only that column; the unchanged encrypted
    // field keeps its ciphertext.
    let ciphertext = stored_ciphertext(id).await;
    loaded.title = "edited".into();
    assert_eq!(loaded.dirty_columns(), ["title"]);
    assert!(loaded.is_dirty());
    loaded.save().await.expect("save title");
    assert_eq!(take_writes().await, ["title"]);
    assert_eq!(stored_ciphertext(id).await, ciphertext);
    assert_eq!(load(id).await.title, "edited");

    // The snapshot is refreshed: saving again writes nothing.
    assert!(!loaded.is_dirty());
    loaded.save().await.expect("save again");
    assert!(take_writes().await.is_empty());

    // Two handles that change different fields both persist, and the second
    // handle is refreshed with the first one's change.
    let mut first = load(id).await;
    let mut second = load(id).await;
    first.body = "body from first".into();
    second.note = Some("note from second".into());
    first.save().await.expect("first save");
    second.save().await.expect("second save");
    assert_eq!(take_writes().await, ["body", "note"]);
    let both = load(id).await;
    assert_eq!(both.body, "body from first");
    assert_eq!(both.note.as_deref(), Some("note from second"));
    assert_eq!(second.body, "body from first");
    assert!(!second.is_dirty());

    // A changed encrypted field is written, encrypted, and reads back.
    second.secret = "rotated".into();
    second.note = None;
    second.save().await.expect("save secret");
    assert_eq!(take_writes().await, ["note", "secret"]);
    assert_ne!(stored_ciphertext(id).await, "rotated");
    let reread = load(id).await;
    assert_eq!(reread.secret, "rotated");
    assert_eq!(reread.note, None);

    // JSON compares canonical values: reordered keys are no change.
    let mut json_post = load(id).await;
    json_post.meta = Json(json!({ "a": { "x": 1, "y": 2 }, "b": 1 }));
    assert!(!json_post.is_dirty());
    json_post.save().await.expect("save reordered JSON");
    assert!(take_writes().await.is_empty());
    json_post.meta = Json(json!({ "a": { "x": 1, "y": 3 }, "b": 1 }));
    assert_eq!(json_post.dirty_columns(), ["meta"]);
    json_post.save().await.expect("save changed JSON");
    assert_eq!(take_writes().await, ["meta"]);
    assert_eq!(load(id).await.meta.0["a"]["y"], 3);

    // Columns the save did not write, and database-managed columns, take the
    // stored values that another writer set meanwhile.
    let mut stale = load(id).await;
    execute(&format!(
        "UPDATE tracked_posts SET body = 'concurrent body', revision = 7 WHERE id = {id}"
    ))
    .await;
    take_writes().await;
    stale.title = "title over concurrent body".into();
    stale.save().await.expect("save over a concurrent change");
    assert_eq!(take_writes().await, ["title"]);
    assert_eq!(stale.body, "concurrent body");
    assert_eq!(stale.revision, 7);
    assert_eq!(load(id).await.body, "concurrent body");

    // mark_all_dirty() restores the full-row overwrite.
    let mut overwrite = load(id).await;
    execute(&format!(
        "UPDATE tracked_posts SET body = 'lost on purpose' WHERE id = {id}"
    ))
    .await;
    take_writes().await;
    overwrite.mark_all_dirty();
    assert!(!overwrite.snapshot.is_tracked());
    assert_eq!(overwrite.dirty_columns(), TRACKED_COLUMNS);
    overwrite.save().await.expect("full-row save");
    assert_eq!(take_writes().await, ALL_WRITES);
    assert_eq!(load(id).await.body, "concurrent body");
    assert!(overwrite.snapshot.is_tracked());

    // A persisted record built in code has no snapshot: every column.
    let mut rebuilt = TrackedPost {
        id,
        title: "rebuilt".into(),
        body: "rebuilt body".into(),
        secret: "rebuilt".into(),
        note: None,
        meta: Json(json!({})),
        revision: 0,
        snapshot: RowSnapshot::default(),
    };
    rebuilt.save().await.expect("save rebuilt record");
    assert_eq!(take_writes().await, ALL_WRITES);
    assert_eq!(
        rebuilt.revision, 7,
        "the refresh reads the database default"
    );

    // update_partial() writes only the patched columns that differ.
    let mut patched = load(id).await;
    patched
        .update_partial()
        .title("patched".into())
        .body("rebuilt body".into())
        .save()
        .await
        .expect("partial update");
    assert_eq!(take_writes().await, ["title"]);
    assert_eq!(patched.title, "patched");
    assert!(patched.snapshot.is_tracked());
    assert!(!patched.is_dirty());

    // A row deleted meanwhile: a changed save reports RecordNotFound and stays
    // dirty; an unchanged one has nothing to write.
    let mut vanished = load(id).await;
    let mut untouched = vanished.clone();
    execute(&format!("DELETE FROM tracked_posts WHERE id = {id}")).await;
    vanished.title = "too late".into();
    let saved = vanished.save().await;
    assert!(matches!(saved, Err(Error::RecordNotFound)), "{saved:?}");
    assert_eq!(vanished.id, id);
    assert_eq!(vanished.dirty_columns(), ["title"]);
    untouched.save().await.expect("nothing to write");
    assert!(take_writes().await.is_empty());

    // Streamed rows are tracked too.
    let mut streamed = TrackedPost {
        id: 0,
        title: "streamed".into(),
        body: "b".into(),
        secret: "s".into(),
        note: None,
        meta: Json(json!(null)),
        revision: 0,
        snapshot: RowSnapshot::default(),
    };
    streamed.save().await.expect("insert streamed post");
    let query = TrackedPost::query();
    let rows: Vec<TrackedPost> = futures::TryStreamExt::try_collect(query.stream())
        .await
        .expect("stream posts");
    assert_eq!(rows.len(), 1);
    assert!(rows[0].snapshot.is_tracked());
    assert!(!rows[0].is_dirty());
}
