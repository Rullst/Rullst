//! Assessment-only schema and materialized regression for the detached profile.

pub(super) const ASSESSMENT_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260828000000_add_assessment"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("quizzes", |table| {
            table.id();
            table.integer("lesson_id").not_null();
            table.string("title").not_null();
            table.integer("passing_score").not_null();
            table.integer("max_attempts").not_null();
            table.string("ruleset_version").not_null();
            table.string("status").not_null();
            table.timestamps();
        }).await?;
        Schema::create("quiz_questions", |table| {
            table.id();
            table.integer("quiz_id").not_null();
            table.string("prompt").not_null();
            table.integer("position").not_null();
            table.integer("points").not_null();
            table.boolean("enabled").not_null();
            table.timestamps();
        }).await?;
        Schema::create("quiz_options", |table| {
            table.id();
            table.integer("question_id").not_null();
            table.string("label").not_null();
            table.integer("position").not_null();
            table.boolean("is_correct").not_null();
            table.timestamps();
        }).await?;
        Schema::create("quiz_attempts", |table| {
            table.id();
            table.string("attempt_key").not_null();
            table.integer("quiz_id").not_null();
            table.integer("actor_user_id").not_null();
            table.integer("subject_user_id").not_null();
            table.string("ruleset_version").not_null();
            table.string("status").not_null();
            table.integer("score_percent").not_null();
            table.integer("points_awarded").not_null();
            table.integer("max_points").not_null();
            table.big_integer("graded_at_epoch").not_null();
            table.timestamps();
        }).await?;
        Schema::create("quiz_answers", |table| {
            table.id();
            table.integer("attempt_id").not_null();
            table.integer("question_id").not_null();
            table.integer("option_id").not_null();
            table.boolean("correct").not_null();
            table.integer("points_awarded").not_null();
            table.timestamps();
        }).await?;

        let pool = Orm::pool()?;
        for statement in [
            "CREATE INDEX quizzes_lesson_status_idx ON quizzes(lesson_id, status)",
            "CREATE UNIQUE INDEX quiz_questions_position_unique ON quiz_questions(quiz_id, position)",
            "CREATE UNIQUE INDEX quiz_options_position_unique ON quiz_options(question_id, position)",
            "CREATE UNIQUE INDEX quiz_attempts_key_unique ON quiz_attempts(attempt_key)",
            "CREATE INDEX quiz_attempts_subject_idx ON quiz_attempts(subject_user_id, quiz_id, ruleset_version, status)",
            "CREATE UNIQUE INDEX quiz_answers_attempt_question_unique ON quiz_answers(attempt_id, question_id)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?;
        }
        for fixture in [
            "INSERT INTO quizzes (id, lesson_id, title, passing_score, max_attempts, ruleset_version, status) VALUES (1, 1, 'Memory Safety Checkpoint', 80, 3, 'memory-rules-v1', 'published')",
            "INSERT INTO quizzes (id, lesson_id, title, passing_score, max_attempts, ruleset_version, status) VALUES (2, 1, 'Borrowing Checkpoint', 80, 3, 'borrowing-rules-v1', 'published')",
            "INSERT INTO quiz_questions (id, quiz_id, prompt, position, points, enabled) VALUES (1, 1, 'Which rule prevents simultaneous mutable aliases?', 1, 100, 1)",
            "INSERT INTO quiz_options (id, question_id, label, position, is_correct) VALUES (1, 1, 'Exclusive mutable borrowing', 1, 1)",
            "INSERT INTO quiz_options (id, question_id, label, position, is_correct) VALUES (2, 1, 'Unchecked shared mutation', 2, 0)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(fixture)).execute(pool).await?;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("quiz_answers").await?;
        Schema::drop_if_exists("quiz_attempts").await?;
        Schema::drop_if_exists("quiz_options").await?;
        Schema::drop_if_exists("quiz_questions").await?;
        Schema::drop_if_exists("quizzes").await
    }
}
"##;

pub(super) const ASSESSMENT_MIGRATIONS_MODULE: &str = r##"pub mod m20260601000000_create_lms_tables;
pub mod m20260827000000_add_learning_access;
pub mod m20260828000000_add_assessment;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260601000000_create_lms_tables::MigrationImpl),
        Box::new(m20260827000000_add_learning_access::MigrationImpl),
        Box::new(m20260828000000_add_assessment::MigrationImpl),
    ]
}

#[cfg(test)]
mod tests {
    use super::get_migrations;
    use crate::services::assessment_service::{
        AssessmentError, QuizAnswerInput, QuizSubmission, grade_quiz_at, quiz_for_learner,
    };
    use rullst::db::Orm;
    use rullst_security::UserContext;

    fn answer(option_id: i32) -> Vec<QuizAnswerInput> {
        vec![QuizAnswerInput { question_id: 1, option_id }]
    }

    fn submission(attempt_key: impl Into<String>, option_id: i32) -> QuizSubmission {
        QuizSubmission {
            attempt_key: attempt_key.into(),
            quiz_id: 1,
            subject_user_id: 7,
            ruleset_version: "memory-rules-v1".to_string(),
            answers: answer(option_id),
        }
    }

    #[tokio::test]
    async fn assessment_is_authoritative_idempotent_bounded_and_owner_scoped() {
        Orm::init("sqlite:file:rullst_detached_assessment?mode=memory&cache=shared")
            .await
            .expect("assessment SQLite should initialize");
        for migration in get_migrations() {
            migration.up().await.expect("assessment migration should run");
        }
        rullst::db::sqlx::query(
            "INSERT INTO users (id, name, email, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(7_i32).bind("Offline Learner").bind("learner@example.test")
        .execute(Orm::pool().expect("assessment pool")).await
        .expect("assessment learner fixture");
        rullst::db::sqlx::query(
            "INSERT INTO enrollments (user_id, course_id, status, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(7_i32).bind(1_i32).bind("active")
        .execute(Orm::pool().expect("assessment pool")).await
        .expect("assessment enrollment fixture");

        let learner = UserContext::new("7", vec!["student".to_string()]);
        let presentation = quiz_for_learner(&learner, 7, 1).await
            .expect("owner assessment presentation");
        assert_eq!(presentation.questions.len(), 1);
        assert_eq!(presentation.questions[0].options.len(), 2);
        let encoded = serde_json::to_string(&presentation).expect("serialize presentation");
        assert!(!encoded.contains("is_correct"));

        let first = submission("detached-attempt-1", 1);
        let grade = grade_quiz_at(&learner, first.clone(), 3_000).await
            .expect("authoritative grade");
        assert!(grade.applied);
        assert!(grade.passed);
        assert_eq!(grade.score_percent, 100);
        let replay = grade_quiz_at(&learner, first.clone(), 3_001).await
            .expect("idempotent replay");
        assert!(!replay.applied);
        assert!(matches!(
            grade_quiz_at(
                &UserContext::new("8", vec!["student".to_string()]),
                first,
                3_002,
            ).await,
            Err(AssessmentError::Access(_))
        ));

        let mut conflict = submission("detached-attempt-1", 1);
        conflict.quiz_id = 2;
        conflict.ruleset_version = "borrowing-rules-v1".to_string();
        assert!(matches!(
            grade_quiz_at(&learner, conflict, 3_003).await,
            Err(AssessmentError::IdempotencyConflict)
        ));
        assert!(matches!(
            grade_quiz_at(&learner, submission("detached-tampered", 999), 3_004).await,
            Err(AssessmentError::InvalidField("unknown option"))
        ));
        for number in 2..=3 {
            let grade = grade_quiz_at(
                &learner,
                submission(format!("detached-attempt-{number}"), 2),
                3_000 + i64::from(number),
            ).await.expect("bounded assessment attempt");
            assert_eq!(grade.score_percent, 0);
        }
        assert!(matches!(
            grade_quiz_at(&learner, submission("detached-attempt-4", 1), 3_010).await,
            Err(AssessmentError::AttemptLimit)
        ));
        let counts = rullst::db::sqlx::query_as::<_, (i64, i64)>(
            "SELECT (SELECT COUNT(*) FROM quiz_attempts), (SELECT COUNT(*) FROM quiz_answers)",
        ).fetch_one(Orm::pool().expect("assessment pool")).await
            .expect("assessment persistence counts");
        assert_eq!(counts, (3, 3));
    }
}
"##;
