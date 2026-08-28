// Read-only Nexus models for assignments, rubrics, submissions and grades.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        ("src/models/assignment.rs", ASSIGNMENT.to_string()),
        (
            "src/models/rubric_criterion.rs",
            RUBRIC_CRITERION.to_string(),
        ),
        (
            "src/models/assignment_submission.rs",
            ASSIGNMENT_SUBMISSION.to_string(),
        ),
        (
            "src/models/assignment_grade.rs",
            ASSIGNMENT_GRADE.to_string(),
        ),
        ("src/models/rubric_score.rs", RUBRIC_SCORE.to_string()),
        (
            "src/models/assignment_grade_correction.rs",
            ASSIGNMENT_GRADE_CORRECTION.to_string(),
        ),
    ]
}

const ASSIGNMENT: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "assignments")]
pub struct Assignment {
    pub id: i32,
    pub lesson_id: i32,
    pub title: String,
    pub instructions: String,
    pub ruleset_version: String,
    pub max_attempts: i32,
    pub due_at_epoch: i64,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for Assignment {
    fn nexus_table() -> &'static str { "assignments" }
    fn nexus_label() -> &'static str { "Assignments" }
    fn nexus_icon() -> &'static str { "📝" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "lesson_id", label: "Lesson", kind: FieldKind::ForeignKey { table: "lessons", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "title", label: "Title", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "instructions", label: "Instructions", kind: FieldKind::Textarea, hidden: false, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "max_attempts", label: "Max Attempts", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "due_at_epoch", label: "Due Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const RUBRIC_CRITERION: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "rubric_criteria")]
pub struct RubricCriterion {
    pub id: i32,
    pub assignment_id: i32,
    pub criterion_key: String,
    pub label: String,
    pub max_points: i32,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for RubricCriterion {
    fn nexus_table() -> &'static str { "rubric_criteria" }
    fn nexus_label() -> &'static str { "Rubric Criteria" }
    fn nexus_icon() -> &'static str { "📏" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "assignment_id", label: "Assignment", kind: FieldKind::ForeignKey { table: "assignments", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "criterion_key", label: "Criterion Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "label", label: "Label", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "max_points", label: "Max Points", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "position", label: "Position", kind: FieldKind::Number, hidden: false, readonly: true },
        ]
    }
}
"##;

const ASSIGNMENT_SUBMISSION: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "assignment_submissions")]
pub struct AssignmentSubmission {
    pub id: i32,
    pub submission_key: String,
    pub assignment_id: i32,
    pub actor_user_id: i32,
    pub subject_user_id: i32,
    pub attempt_number: i32,
    pub content_text: String,
    pub ruleset_version: String,
    pub status: String,
    pub submitted_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for AssignmentSubmission {
    fn nexus_table() -> &'static str { "assignment_submissions" }
    fn nexus_label() -> &'static str { "Assignment Submissions" }
    fn nexus_icon() -> &'static str { "📨" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "submission_key", label: "Submission Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "assignment_id", label: "Assignment", kind: FieldKind::ForeignKey { table: "assignments", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Actor", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "attempt_number", label: "Attempt", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "content_text", label: "Submission", kind: FieldKind::Textarea, hidden: true, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "submitted_at_epoch", label: "Submitted Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
        ]
    }
}
"##;

const ASSIGNMENT_GRADE: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "assignment_grades")]
pub struct AssignmentGrade {
    pub id: i32,
    pub grading_key: String,
    pub assignment_id: i32,
    pub submission_id: i32,
    pub grader_user_id: i32,
    pub subject_user_id: i32,
    pub points_awarded: i32,
    pub max_points: i32,
    pub feedback: String,
    pub ruleset_version: String,
    pub request_json: String,
    pub graded_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for AssignmentGrade {
    fn nexus_table() -> &'static str { "assignment_grades" }
    fn nexus_label() -> &'static str { "Assignment Grades" }
    fn nexus_icon() -> &'static str { "✅" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "grading_key", label: "Grading Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "assignment_id", label: "Assignment", kind: FieldKind::ForeignKey { table: "assignments", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "submission_id", label: "Submission", kind: FieldKind::ForeignKey { table: "assignment_submissions", label_col: "submission_key" }, hidden: false, readonly: true },
            FieldMeta { name: "grader_user_id", label: "Evaluator", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "points_awarded", label: "Points", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "max_points", label: "Maximum", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "feedback", label: "Feedback", kind: FieldKind::Textarea, hidden: false, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "request_json", label: "Canonical Request", kind: FieldKind::Json, hidden: true, readonly: true },
            FieldMeta { name: "graded_at_epoch", label: "Graded Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
        ]
    }
}
"##;

const RUBRIC_SCORE: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "rubric_scores")]
pub struct RubricScore {
    pub id: i32,
    pub assignment_grade_id: i32,
    pub criterion_id: i32,
    pub points_awarded: i32,
    pub feedback: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for RubricScore {
    fn nexus_table() -> &'static str { "rubric_scores" }
    fn nexus_label() -> &'static str { "Rubric Scores" }
    fn nexus_icon() -> &'static str { "📊" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "assignment_grade_id", label: "Grade", kind: FieldKind::ForeignKey { table: "assignment_grades", label_col: "grading_key" }, hidden: false, readonly: true },
            FieldMeta { name: "criterion_id", label: "Criterion", kind: FieldKind::ForeignKey { table: "rubric_criteria", label_col: "label" }, hidden: false, readonly: true },
            FieldMeta { name: "points_awarded", label: "Points", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "feedback", label: "Feedback", kind: FieldKind::Textarea, hidden: false, readonly: true },
        ]
    }
}
"##;

const ASSIGNMENT_GRADE_CORRECTION: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "assignment_grade_corrections")]
pub struct AssignmentGradeCorrection {
    pub id: i32,
    pub correction_key: String,
    pub assignment_grade_id: i32,
    pub actor_user_id: i32,
    pub previous_points: i32,
    pub corrected_points: i32,
    pub max_points: i32,
    pub reason: String,
    pub scores_json: String,
    pub request_json: String,
    pub corrected_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for AssignmentGradeCorrection {
    fn nexus_table() -> &'static str { "assignment_grade_corrections" }
    fn nexus_label() -> &'static str { "Assignment Grade Corrections" }
    fn nexus_icon() -> &'static str { "🧾" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "correction_key", label: "Correction Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "assignment_grade_id", label: "Grade", kind: FieldKind::ForeignKey { table: "assignment_grades", label_col: "grading_key" }, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Administrator", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "previous_points", label: "Before", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "corrected_points", label: "After", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "max_points", label: "Maximum", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "reason", label: "Reason", kind: FieldKind::Textarea, hidden: false, readonly: true },
            FieldMeta { name: "scores_json", label: "Criterion Scores", kind: FieldKind::Json, hidden: false, readonly: true },
            FieldMeta { name: "request_json", label: "Canonical Request", kind: FieldKind::Json, hidden: true, readonly: true },
            FieldMeta { name: "corrected_at_epoch", label: "Corrected Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
        ]
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::{
        ASSIGNMENT, ASSIGNMENT_GRADE, ASSIGNMENT_GRADE_CORRECTION, ASSIGNMENT_SUBMISSION,
        RUBRIC_CRITERION, RUBRIC_SCORE,
    };

    #[test]
    fn assignment_records_are_read_only_and_hide_submission_content() {
        for model in [
            ASSIGNMENT,
            RUBRIC_CRITERION,
            ASSIGNMENT_SUBMISSION,
            ASSIGNMENT_GRADE,
            ASSIGNMENT_GRADE_CORRECTION,
            RUBRIC_SCORE,
        ] {
            assert!(model.contains("readonly: true"));
        }
        assert!(ASSIGNMENT_SUBMISSION.contains("content_text"));
        assert!(ASSIGNMENT_SUBMISSION.contains("hidden: true"));
        assert!(ASSIGNMENT_GRADE.contains("request_json"));
        assert!(ASSIGNMENT_GRADE_CORRECTION.contains("previous_points"));
        assert!(ASSIGNMENT_GRADE_CORRECTION.contains("corrected_points"));
    }
}
