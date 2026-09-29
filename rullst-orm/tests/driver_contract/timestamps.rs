//! `Blueprint::timestamps()` and TEXT defaults must be accepted by every driver.

use rullst_orm::schema::{Blueprint, ColumnDefault, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_timestamped_notes")]
struct TimestampedNote {
    id: i32,
    title: String,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_timestamped_notes")]
struct TimestampedNoteView {
    id: i32,
    title: String,
    status: String,
    created_at: Option<String>,
    updated_at: Option<String>,
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_timestamped_notes", |table: &mut Blueprint| {
        table.id();
        table.string("title").not_null();
        table
            .string("status")
            .not_null()
            .default(ColumnDefault::Text("draft".to_string()));
        table.timestamps();
    })
    .await
    .unwrap_or_else(|error| panic!("{driver} must accept timestamps() and TEXT defaults: {error}"));

    let mut note = TimestampedNote {
        id: 0,
        title: "defaults".to_string(),
    };
    note.save()
        .await
        .expect("insert relying on column defaults");
    let stored = TimestampedNoteView::find(note.id)
        .await
        .expect("read defaulted row")
        .expect("defaulted row exists");
    assert_eq!(stored.id, note.id);
    assert_eq!(stored.title, "defaults");
    assert_eq!(stored.status, "draft");
    assert!(
        stored
            .created_at
            .as_deref()
            .is_some_and(|value| !value.is_empty()),
        "{driver} must fill created_at"
    );
    assert!(
        stored
            .updated_at
            .as_deref()
            .is_some_and(|value| !value.is_empty()),
        "{driver} must fill updated_at"
    );

    Schema::drop_if_exists("contract_timestamped_notes")
        .await
        .expect("drop timestamp contract table");
}
