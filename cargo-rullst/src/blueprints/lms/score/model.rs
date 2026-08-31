//! Read-only generated score-event model.

pub(super) const SCORE_EVENT_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "score_events")]
pub struct ScoreEvent {
    pub id: i32,
    pub idempotency_key: String,
    pub schema_version: i32,
    pub origin: String,
    pub actor_user_id: i32,
    pub subject_user_id: i32,
    pub course_id: i32,
    pub activity_id: i32,
    pub attempt_key: String,
    pub points: i32,
    pub max_score: i32,
    pub occurred_at: String,
    pub ruleset_version: String,
    pub evidence_sha256: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for ScoreEvent {
    fn nexus_table() -> &'static str { "score_events" }
    fn nexus_label() -> &'static str { "Score Events" }
    fn nexus_icon() -> &'static str { "🧾" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "idempotency_key", label: "Idempotency Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "schema_version", label: "Schema Version", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "origin", label: "Origin", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Actor", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "activity_id", label: "Activity", kind: FieldKind::ForeignKey { table: "activities", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "attempt_key", label: "Attempt", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "points", label: "Points", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "max_score", label: "Maximum Score", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "occurred_at", label: "Occurred At", kind: FieldKind::DateTime, hidden: false, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "evidence_sha256", label: "Evidence SHA-256", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;
