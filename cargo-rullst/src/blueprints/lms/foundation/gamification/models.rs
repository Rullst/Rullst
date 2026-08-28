//! Bounded score and leaderboard models for the detached gamification profile.

pub(super) fn files() -> Vec<(&'static str, String)> {
    vec![
        ("src/models/activity.rs", ACTIVITY.to_string()),
        ("src/models/score_event.rs", SCORE_EVENT.to_string()),
        (
            "src/models/leaderboard_entry.rs",
            LEADERBOARD_ENTRY.to_string(),
        ),
    ]
}

const ACTIVITY: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "activities")]
pub struct Activity {
    pub id: i32,
    pub lesson_id: i32,
    pub title: String,
    pub activity_kind: String,
    pub max_score: i32,
    pub max_attempts: i32,
    pub ruleset_version: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for Activity {
    fn nexus_table() -> &'static str { "activities" }
    fn nexus_label() -> &'static str { "Activities" }
    fn nexus_icon() -> &'static str { "🎯" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "lesson_id", label: "Lesson", kind: FieldKind::ForeignKey { table: "lessons", label_col: "title" }, hidden: false, readonly: false },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "activity_kind", label: "Kind", kind: FieldKind::Enum { options: vec!["exercise", "game"] }, hidden: false, readonly: false },
            FieldMeta { name: "max_score", label: "Maximum Score", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "max_attempts", label: "Maximum Attempts", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Enum { options: vec!["draft", "published", "archived"] }, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const SCORE_EVENT: &str = r##"use rullst::db::{FromRow, Orm};
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
    pub ruleset_version: String,
    pub season_key: String,
    pub evidence_digest: String,
    pub occurred_at_epoch: i64,
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
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "season_key", label: "Season", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "evidence_digest", label: "Evidence Digest", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "occurred_at_epoch", label: "Occurred Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const LEADERBOARD_ENTRY: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm, serde::Serialize)]
#[orm(table = "leaderboard_entries")]
pub struct LeaderboardEntry {
    pub id: i32,
    pub user_id: i32,
    pub course_id: i32,
    pub season_key: String,
    pub score: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for LeaderboardEntry {
    fn nexus_table() -> &'static str { "leaderboard_entries" }
    fn nexus_label() -> &'static str { "Leaderboard" }
    fn nexus_icon() -> &'static str { "🥇" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "season_key", label: "Season", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "score", label: "Authoritative Score", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;
