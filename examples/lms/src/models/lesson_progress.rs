use rullst::db::{Orm, RowSnapshot};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};
use rullst::orm::Timestamp;

#[derive(Debug, Clone, Orm)]
#[orm(table = "lesson_progress")]
pub struct LessonProgress {
    pub id: i32,
    pub user_id: i32,
    pub lesson_id: i32,
    pub progress_percent: i32,
    pub completed: i32,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}

impl LessonProgress {
    pub async fn for_learner(
        user_id: i32,
        lesson_id: i32,
    ) -> Result<Option<Self>, rullst_orm::Error> {
        Self::query()
            .where_eq("user_id", user_id)
            .where_eq("lesson_id", lesson_id)
            .first()
            .await
    }
}

impl NexusModel for LessonProgress {
    fn nexus_table() -> &'static str {
        "lesson_progress"
    }
    fn nexus_label() -> &'static str {
        "Lesson Progress"
    }
    fn nexus_icon() -> &'static str {
        "📈"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number)
                .hidden()
                .readonly(),
            FieldMeta::new(
                "user_id",
                "Learner",
                FieldKind::ForeignKey {
                    table: "users",
                    label_col: "email",
                },
            )
            .readonly(),
            FieldMeta::new(
                "lesson_id",
                "Lesson",
                FieldKind::ForeignKey {
                    table: "lessons",
                    label_col: "title",
                },
            )
            .readonly(),
            FieldMeta::new("progress_percent", "Progress %", FieldKind::Number),
            FieldMeta::new("completed", "Completed", FieldKind::Number),
            FieldMeta::new("created_at", "Created At", FieldKind::DateTimeUtc).readonly(),
            FieldMeta::new("updated_at", "Updated At", FieldKind::DateTimeUtc).readonly(),
        ]
    }
}
