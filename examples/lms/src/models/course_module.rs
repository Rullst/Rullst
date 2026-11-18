use rullst::db::{Orm, RowSnapshot};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};
use rullst::orm::Timestamp;

#[derive(Debug, Clone, Orm)]
#[orm(table = "course_modules")]
pub struct CourseModule {
    pub id: i32,
    pub course_id: i32,
    pub title: String,
    pub position: i32,
    pub status: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}

impl NexusModel for CourseModule {
    fn nexus_table() -> &'static str {
        "course_modules"
    }
    fn nexus_label() -> &'static str {
        "Course Modules"
    }
    fn nexus_icon() -> &'static str {
        "🧭"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number)
                .hidden()
                .readonly(),
            FieldMeta::new(
                "course_id",
                "Course",
                FieldKind::ForeignKey {
                    table: "courses",
                    label_col: "title",
                },
            ),
            FieldMeta::new("title", "Title", FieldKind::Text),
            FieldMeta::new("position", "Position", FieldKind::Number),
            FieldMeta::new(
                "status",
                "Status",
                FieldKind::Enum {
                    options: vec!["draft", "published", "archived"],
                },
            ),
            FieldMeta::new("created_at", "Created At", FieldKind::DateTimeUtc).readonly(),
            FieldMeta::new("updated_at", "Updated At", FieldKind::DateTimeUtc).readonly(),
        ]
    }
}
