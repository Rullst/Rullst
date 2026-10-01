use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "lesson_progress")]
pub struct LessonProgress {
    pub id: i32,
    pub user_id: i32,
    pub lesson_id: i32,
    pub progress_percent: i32,
    pub completed: i32,
    pub created_at: String,
    pub updated_at: String,
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
    fn nexus_table() -> &'static str { "lesson_progress" }
    fn nexus_label() -> &'static str { "Lesson Progress" }
    fn nexus_icon() -> &'static str { "📈" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "lesson_id", label: "Lesson", kind: FieldKind::ForeignKey { table: "lessons", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "progress_percent", label: "Progress %", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "completed", label: "Completed", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
