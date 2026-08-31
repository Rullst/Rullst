// Curriculum model templates for the LMS starter.

#[path = "publication.rs"]
mod publication;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = vec![
        ("src/models/course_module.rs", COURSE_MODULE.to_string()),
        ("src/models/quiz.rs", QUIZ.to_string()),
        ("src/models/quiz_question.rs", QUIZ_QUESTION.to_string()),
        ("src/models/quiz_option.rs", QUIZ_OPTION.to_string()),
        ("src/models/quiz_attempt.rs", QUIZ_ATTEMPT.to_string()),
        ("src/models/quiz_answer.rs", QUIZ_ANSWER.to_string()),
        ("src/models/activity.rs", ACTIVITY.to_string()),
    ];
    files.extend(publication::get_files());
    files
}

const COURSE_MODULE: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "course_modules")]
pub struct CourseModule {
    pub id: i32,
    pub course_id: i32,
    pub title: String,
    pub position: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for CourseModule {
    fn nexus_table() -> &'static str { "course_modules" }
    fn nexus_label() -> &'static str { "Course Modules" }
    fn nexus_icon() -> &'static str { "🧭" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: false },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "position", label: "Position", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Enum { options: vec!["draft", "published", "archived"] }, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const QUIZ: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "quizzes")]
pub struct Quiz {
    pub id: i32,
    pub lesson_id: i32,
    pub activity_id: i32,
    pub title: String,
    pub passing_score: i32,
    pub max_attempts: i32,
    pub time_limit_seconds: i32,
    pub ruleset_version: String,
    pub season_key: String,
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
            FieldMeta { name: "activity_id", label: "Score Activity", kind: FieldKind::ForeignKey { table: "activities", label_col: "title" }, hidden: false, readonly: false },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "passing_score", label: "Passing Score", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "max_attempts", label: "Maximum Attempts", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "time_limit_seconds", label: "Time Limit Seconds", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "ruleset_version", label: "Ruleset Version", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "season_key", label: "Leaderboard Season", kind: FieldKind::Text, hidden: false, readonly: false },
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
    pub ruleset_version: String,
    pub season_key: String,
    pub evidence_sha256: String,
    pub config_json: String,
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
            FieldMeta { name: "activity_kind", label: "Kind", kind: FieldKind::Enum { options: vec!["quiz", "exercise", "project", "game"] }, hidden: false, readonly: false },
            FieldMeta { name: "max_score", label: "Maximum Score", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "season_key", label: "Season", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "evidence_sha256", label: "Evidence SHA-256", kind: FieldKind::Text, hidden: false, readonly: false },
            FieldMeta { name: "config_json", label: "Versioned Configuration", kind: FieldKind::Json, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::get_files;

    #[test]
    fn curriculum_templates_keep_explicit_parent_boundaries() {
        let files = get_files();
        let rendered = files
            .iter()
            .map(|(_, source)| source.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("pub course_id: i32"));
        assert!(rendered.contains("pub lesson_id: i32"));
        assert!(rendered.contains("Versioned Configuration"));
    }
}
