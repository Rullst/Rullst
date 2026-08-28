//! Bounded assessment models emitted by the detached profile.

pub(super) fn files() -> Vec<(&'static str, String)> {
    vec![
        ("src/models/quiz.rs", QUIZ.to_string()),
        ("src/models/quiz_question.rs", QUIZ_QUESTION.to_string()),
        ("src/models/quiz_option.rs", QUIZ_OPTION.to_string()),
        ("src/models/quiz_attempt.rs", QUIZ_ATTEMPT.to_string()),
        ("src/models/quiz_answer.rs", QUIZ_ANSWER.to_string()),
    ]
}

const QUIZ: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "quizzes")]
pub struct Quiz {
    pub id: i32,
    pub lesson_id: i32,
    pub title: String,
    pub passing_score: i32,
    pub max_attempts: i32,
    pub ruleset_version: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for Quiz {
    fn nexus_table() -> &'static str { "quizzes" }
    fn nexus_label() -> &'static str { "Quizzes" }
    fn nexus_icon() -> &'static str { "🧠" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "lesson_id", label: "Lesson", kind: FieldKind::ForeignKey { table: "lessons", label_col: "title" }, hidden: false, readonly: false },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "passing_score", label: "Passing Score", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "max_attempts", label: "Maximum Attempts", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "ruleset_version", label: "Ruleset Version", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Enum { options: vec!["draft", "published", "archived"] }, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const QUIZ_QUESTION: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "quiz_questions")]
pub struct QuizQuestion {
    pub id: i32,
    pub quiz_id: i32,
    pub prompt: String,
    pub position: i32,
    pub points: i32,
    pub enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for QuizQuestion {
    fn nexus_table() -> &'static str { "quiz_questions" }
    fn nexus_label() -> &'static str { "Quiz Questions" }
    fn nexus_icon() -> &'static str { "❓" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "quiz_id", label: "Quiz", kind: FieldKind::ForeignKey { table: "quizzes", label_col: "title" }, hidden: false, readonly: false },
            FieldMeta { name: "prompt", label: "Prompt", kind: FieldKind::Textarea, hidden: false, readonly: false },
            FieldMeta { name: "position", label: "Position", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "points", label: "Points", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "enabled", label: "Enabled", kind: FieldKind::Boolean, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const QUIZ_OPTION: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "quiz_options")]
pub struct QuizOption {
    pub id: i32,
    pub question_id: i32,
    pub label: String,
    pub position: i32,
    pub is_correct: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for QuizOption {
    fn nexus_table() -> &'static str { "quiz_options" }
    fn nexus_label() -> &'static str { "Quiz Options" }
    fn nexus_icon() -> &'static str { "☑️" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "question_id", label: "Question", kind: FieldKind::ForeignKey { table: "quiz_questions", label_col: "prompt" }, hidden: false, readonly: false },
            FieldMeta { name: "label", label: "Label", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "position", label: "Position", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "is_correct", label: "Correct", kind: FieldKind::Boolean, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const QUIZ_ATTEMPT: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "quiz_attempts")]
pub struct QuizAttempt {
    pub id: i32,
    pub attempt_key: String,
    pub quiz_id: i32,
    pub actor_user_id: i32,
    pub subject_user_id: i32,
    pub ruleset_version: String,
    pub status: String,
    pub score_percent: i32,
    pub points_awarded: i32,
    pub max_points: i32,
    pub graded_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for QuizAttempt {
    fn nexus_table() -> &'static str { "quiz_attempts" }
    fn nexus_label() -> &'static str { "Quiz Attempts" }
    fn nexus_icon() -> &'static str { "📝" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "attempt_key", label: "Attempt Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "quiz_id", label: "Quiz", kind: FieldKind::ForeignKey { table: "quizzes", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Actor", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "score_percent", label: "Score %", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "points_awarded", label: "Points", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "max_points", label: "Maximum", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "graded_at_epoch", label: "Graded Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const QUIZ_ANSWER: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "quiz_answers")]
pub struct QuizAnswer {
    pub id: i32,
    pub attempt_id: i32,
    pub question_id: i32,
    pub option_id: i32,
    pub correct: i32,
    pub points_awarded: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for QuizAnswer {
    fn nexus_table() -> &'static str { "quiz_answers" }
    fn nexus_label() -> &'static str { "Quiz Answers" }
    fn nexus_icon() -> &'static str { "🔎" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "attempt_id", label: "Attempt", kind: FieldKind::ForeignKey { table: "quiz_attempts", label_col: "attempt_key" }, hidden: false, readonly: true },
            FieldMeta { name: "question_id", label: "Question", kind: FieldKind::ForeignKey { table: "quiz_questions", label_col: "prompt" }, hidden: false, readonly: true },
            FieldMeta { name: "option_id", label: "Selected Option", kind: FieldKind::ForeignKey { table: "quiz_options", label_col: "label" }, hidden: false, readonly: true },
            FieldMeta { name: "correct", label: "Correct", kind: FieldKind::Boolean, hidden: false, readonly: true },
            FieldMeta { name: "points_awarded", label: "Points", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;
