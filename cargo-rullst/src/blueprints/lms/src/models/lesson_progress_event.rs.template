use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "lesson_progress_events")]
pub struct LessonProgressEvent {
    pub id: i32,
    pub event_key: String,
    pub actor_user_id: i32,
    pub subject_user_id: i32,
    pub lesson_id: i32,
    pub previous_percent: i32,
    pub current_percent: i32,
    pub event_kind: String,
    pub reason: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for LessonProgressEvent {
    fn nexus_table() -> &'static str { "lesson_progress_events" }
    fn nexus_label() -> &'static str { "Progress Audit" }
    fn nexus_icon() -> &'static str { "🧾" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "event_key", label: "Event Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Actor", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "lesson_id", label: "Lesson", kind: FieldKind::ForeignKey { table: "lessons", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "previous_percent", label: "Previous %", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "current_percent", label: "Current %", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "event_kind", label: "Kind", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "reason", label: "Reason", kind: FieldKind::Textarea, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
