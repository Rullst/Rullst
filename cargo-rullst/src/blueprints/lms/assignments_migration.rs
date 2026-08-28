// Reversible assignment, rubric, submission and grade schema.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/migrations/m20260905000000_add_assignments.rs",
        ASSIGNMENTS_MIGRATION.to_string(),
    )]
}

const ASSIGNMENTS_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m20260905000000_add_assignments" }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("assignments", |table| {
            table.id(); table.integer("lesson_id").not_null(); table.string("title").not_null();
            table.string("instructions").not_null(); table.string("ruleset_version").not_null();
            table.integer("max_attempts").not_null(); table.big_integer("due_at_epoch").not_null();
            table.string("status").not_null(); table.timestamps();
        }).await?;
        Schema::create("rubric_criteria", |table| {
            table.id(); table.integer("assignment_id").not_null(); table.string("criterion_key").not_null();
            table.string("label").not_null(); table.integer("max_points").not_null();
            table.integer("position").not_null(); table.timestamps();
        }).await?;
        Schema::create("assignment_submissions", |table| {
            table.id(); table.string("submission_key").not_null(); table.integer("assignment_id").not_null();
            table.integer("actor_user_id").not_null(); table.integer("subject_user_id").not_null();
            table.integer("attempt_number").not_null(); table.string("content_text").not_null();
            table.string("ruleset_version").not_null(); table.string("status").not_null();
            table.big_integer("submitted_at_epoch").not_null(); table.timestamps();
        }).await?;
        Schema::create("assignment_grades", |table| {
            table.id(); table.string("grading_key").not_null(); table.integer("assignment_id").not_null();
            table.integer("submission_id").not_null(); table.integer("grader_user_id").not_null();
            table.integer("subject_user_id").not_null(); table.integer("points_awarded").not_null();
            table.integer("max_points").not_null(); table.string("feedback").not_null();
            table.string("ruleset_version").not_null(); table.string("request_json").not_null();
            table.big_integer("graded_at_epoch").not_null(); table.timestamps();
        }).await?;
        Schema::create("rubric_scores", |table| {
            table.id(); table.integer("assignment_grade_id").not_null(); table.integer("criterion_id").not_null();
            table.integer("points_awarded").not_null(); table.string("feedback").not_null();
            table.timestamps();
        }).await?;
        Schema::create("assignment_grade_corrections", |table| {
            table.id(); table.string("correction_key").not_null();
            table.integer("assignment_grade_id").not_null(); table.integer("actor_user_id").not_null();
            table.integer("previous_points").not_null(); table.integer("corrected_points").not_null();
            table.integer("max_points").not_null(); table.string("reason").not_null();
            table.string("scores_json").not_null(); table.string("request_json").not_null();
            table.big_integer("corrected_at_epoch").not_null(); table.timestamps();
        }).await?;
        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX assignments_lesson_ruleset_unique ON assignments(lesson_id, ruleset_version)",
            "CREATE INDEX assignments_lesson_status_idx ON assignments(lesson_id, status, due_at_epoch)",
            "CREATE UNIQUE INDEX rubric_criteria_key_unique ON rubric_criteria(assignment_id, criterion_key)",
            "CREATE UNIQUE INDEX rubric_criteria_position_unique ON rubric_criteria(assignment_id, position)",
            "CREATE UNIQUE INDEX assignment_submissions_key_unique ON assignment_submissions(submission_key)",
            "CREATE UNIQUE INDEX assignment_submissions_attempt_unique ON assignment_submissions(assignment_id, subject_user_id, attempt_number)",
            "CREATE INDEX assignment_submissions_subject_idx ON assignment_submissions(subject_user_id, assignment_id, status)",
            "CREATE UNIQUE INDEX assignment_grades_key_unique ON assignment_grades(grading_key)",
            "CREATE UNIQUE INDEX assignment_grades_submission_unique ON assignment_grades(submission_id)",
            "CREATE INDEX assignment_grades_subject_idx ON assignment_grades(subject_user_id, assignment_id)",
            "CREATE UNIQUE INDEX rubric_scores_grade_criterion_unique ON rubric_scores(assignment_grade_id, criterion_id)",
            "CREATE UNIQUE INDEX assignment_grade_corrections_key_unique ON assignment_grade_corrections(correction_key)",
            "CREATE INDEX assignment_grade_corrections_grade_idx ON assignment_grade_corrections(assignment_grade_id, id)",
        ] { sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?; }
        for fixture in [
            "INSERT INTO assignments (id, lesson_id, title, instructions, ruleset_version, max_attempts, due_at_epoch, status, created_at, updated_at) VALUES (1, 1, 'Memory Safety Incident Report', 'Explain the ownership failure and propose a safe correction.', 'assignment-memory-v1', 2, 0, 'published', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO rubric_criteria (id, assignment_id, criterion_key, label, max_points, position, created_at, updated_at) VALUES (1, 1, 'analysis', 'Root-cause analysis', 60, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO rubric_criteria (id, assignment_id, criterion_key, label, max_points, position, created_at, updated_at) VALUES (2, 1, 'remediation', 'Safe remediation', 40, 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        ] { sqlx::query(sqlx::AssertSqlSafe(fixture)).execute(pool).await?; }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("assignment_grade_corrections").await?;
        Schema::drop_if_exists("rubric_scores").await?;
        Schema::drop_if_exists("assignment_grades").await?;
        Schema::drop_if_exists("assignment_submissions").await?;
        Schema::drop_if_exists("rubric_criteria").await?;
        Schema::drop_if_exists("assignments").await
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::ASSIGNMENTS_MIGRATION;

    #[test]
    fn assignment_schema_is_reversible_and_deduplicated() {
        assert!(ASSIGNMENTS_MIGRATION.contains("assignment_submissions_attempt_unique"));
        assert!(ASSIGNMENTS_MIGRATION.contains("assignment_grades_submission_unique"));
        assert!(ASSIGNMENTS_MIGRATION.contains("rubric_scores_grade_criterion_unique"));
        assert!(ASSIGNMENTS_MIGRATION.contains("assignment_grade_corrections_key_unique"));
        assert!(ASSIGNMENTS_MIGRATION.contains("drop_if_exists(\"assignments\")"));
    }
}
