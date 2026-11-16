use rullst::db::{Orm, RowSnapshot};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};
use rullst::orm::chrono::{DateTime, Utc};

#[derive(Debug, Clone, Orm)]
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}

impl NexusModel for LessonProgressEvent {
    fn nexus_table() -> &'static str {
        "lesson_progress_events"
    }
    fn nexus_label() -> &'static str {
        "Progress Audit"
    }
    fn nexus_icon() -> &'static str {
        "🧾"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number)
                .hidden()
                .readonly(),
            FieldMeta::new("event_key", "Event Key", FieldKind::Text).readonly(),
            FieldMeta::new(
                "actor_user_id",
                "Actor",
                FieldKind::ForeignKey {
                    table: "users",
                    label_col: "email",
                },
            )
            .readonly(),
            FieldMeta::new(
                "subject_user_id",
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
            FieldMeta::new("previous_percent", "Previous %", FieldKind::Number).readonly(),
            FieldMeta::new("current_percent", "Current %", FieldKind::Number).readonly(),
            FieldMeta::new("event_kind", "Kind", FieldKind::Text).readonly(),
            FieldMeta::new("reason", "Reason", FieldKind::Textarea).readonly(),
            FieldMeta::new("created_at", "Created At", FieldKind::DateTimeUtc).readonly(),
            FieldMeta::new("updated_at", "Updated At", FieldKind::DateTimeUtc).readonly(),
        ]
    }
}
