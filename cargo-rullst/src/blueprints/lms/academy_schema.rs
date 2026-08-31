// Versioned schema extension for curriculum, gamification and automation.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/migrations/m20260828000000_add_academy_domains.rs",
        MIGRATION.to_string(),
    )]
}

pub fn migrations_module() -> String {
    format!(
        "{MIGRATIONS_MODULE}\n{}{}{}{}{}{}{}{}{}{}{}",
        super::academy_schema_tests::GENERATED_TESTS_PREFIX,
        super::academy_score_quiz_tests::GENERATED_SCORE_QUIZ_TESTS_SUFFIX,
        super::academy_notification_realtime_tests::GENERATED_NOTIFICATION_REALTIME_TESTS_SUFFIX,
        super::academy_timed_tests::GENERATED_TESTS_SUFFIX,
        super::academy_http_tests::GENERATED_HTTP_TESTS_SUFFIX,
        super::academy_http_tests::GENERATED_ROLLBACK_TESTS_SUFFIX,
        super::academy_http_tests::GENERATED_ASSIGNMENT_TESTS_SUFFIX,
        super::academy_tenancy_tests::GENERATED_TENANCY_TESTS_SUFFIX,
        super::academy_privacy_tests::GENERATED_PRIVACY_TESTS_SUFFIX,
        super::academy_catalog_tests::GENERATED_CATALOG_TESTS_SUFFIX,
        super::academy_http_tests::academy_completion_tests::GENERATED_COMPLETION_TESTS_SUFFIX,
    )
}

const MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260828000000_add_academy_domains"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("quizzes", |table| {
            table.id();
            table.integer("lesson_id").not_null();
            table.integer("activity_id").not_null();
            table.string("title").not_null();
            table.integer("passing_score").not_null();
            table.integer("max_attempts").not_null();
            table.integer("time_limit_seconds").not_null();
            table.string("ruleset_version").not_null();
            table.string("season_key").not_null();
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

        Schema::create("quiz_attempt_sessions", |table| {
            table.id();
            table.string("attempt_key").not_null();
            table.integer("quiz_id").not_null();
            table.integer("actor_user_id").not_null();
            table.integer("subject_user_id").not_null();
            table.string("ruleset_version").not_null();
            table.string("status").not_null();
            table.big_integer("started_at_epoch").not_null();
            table.big_integer("expires_at_epoch").not_null();
            table.string("presentation_json").not_null();
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

        Schema::create("activities", |table| {
            table.id();
            table.integer("lesson_id").not_null();
            table.string("title").not_null();
            table.string("activity_kind").not_null();
            table.integer("max_score").not_null();
            table.string("ruleset_version").not_null();
            table.string("season_key").not_null();
            table.string("evidence_sha256").not_null();
            table.string("config_json").not_null();
            table.timestamps();
        }).await?;

        Schema::create("activity_attempts", |table| {
            table.id();
            table.string("attempt_key").not_null();
            table.integer("activity_id").not_null();
            table.integer("actor_user_id").not_null();
            table.integer("subject_user_id").not_null();
            table.string("activity_kind").not_null();
            table.string("ruleset_version").not_null();
            table.string("state_json").not_null();
            table.string("submission_key").not_null();
            table.integer("points").not_null();
            table.integer("max_score").not_null();
            table.big_integer("started_at_epoch").not_null();
            table.big_integer("finished_at_epoch").not_null();
            table.string("evidence_sha256").not_null();
            table.timestamps();
        }).await?;

        Schema::create("achievements", |table| {
            table.id();
            table.string("code").not_null();
            table.string("name").not_null();
            table.string("description").not_null();
            table.integer("xp_reward").not_null();
            table.boolean("enabled").not_null();
            table.timestamps();
        }).await?;

        Schema::create("leaderboard_entries", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.integer("course_id").not_null();
            table.string("season_key").not_null();
            table.integer("score").not_null();
            table.timestamps();
        }).await?;

        Schema::create("automation_rules", |table| {
            table.id();
            table.integer("school_id").not_null();
            table.string("name").not_null();
            table.string("trigger_kind").not_null();
            table.string("action_kind").not_null();
            table.string("config_json").not_null();
            table.boolean("enabled").not_null();
            table.timestamps();
        }).await?;

        Schema::create("user_achievements", |table| {
            table.id();
            table.integer("school_id").not_null();
            table.integer("user_id").not_null();
            table.integer("achievement_id").not_null();
            table.string("source_event_key").not_null();
            table.integer("awarded_by_user_id").not_null();
            table.string("awarded_at").not_null();
            table.timestamps();
        }).await?;

        Schema::create("automation_executions", |table| {
            table.id();
            table.integer("school_id").not_null();
            table.string("execution_key").not_null();
            table.integer("rule_id").not_null();
            table.string("source_event_key").not_null();
            table.integer("actor_user_id").not_null();
            table.integer("subject_user_id").not_null();
            table.string("action_kind").not_null();
            table.string("outcome").not_null();
            table.timestamps();
        }).await?;

        Schema::create("score_events", |table| {
            table.id();
            table.string("idempotency_key").not_null();
            table.integer("schema_version").not_null();
            table.string("origin").not_null();
            table.integer("actor_user_id").not_null();
            table.integer("subject_user_id").not_null();
            table.integer("course_id").not_null();
            table.integer("activity_id").not_null();
            table.string("attempt_key").not_null();
            table.integer("points").not_null();
            table.integer("max_score").not_null();
            table.string("occurred_at").not_null();
            table.string("ruleset_version").not_null();
            table.string("evidence_sha256").not_null();
            table.timestamps();
        }).await?;

        Schema::create("score_corrections", |table| {
            table.id();
            table.string("correction_key").not_null();
            table.integer("actor_user_id").not_null();
            table.integer("subject_user_id").not_null();
            table.integer("course_id").not_null();
            table.string("season_key").not_null();
            table.integer("previous_score").not_null();
            table.integer("corrected_score").not_null();
            table.string("reason").not_null();
            table.string("ruleset_version").not_null();
            table.string("occurred_at").not_null();
            table.timestamps();
        }).await?;

        Schema::create("academy_outbox", |table| {
            table.id();
            table.integer("school_id").not_null();
            table.string("event_key").not_null();
            table.string("event_kind").not_null();
            table.integer("subject_user_id").not_null();
            table.string("payload_json").not_null();
            table.string("status").not_null();
            table.integer("attempts").not_null();
            table.string("claimed_by").not_null();
            table.string("claim_key").not_null();
            table.string("last_error").not_null();
            table.string("available_at").not_null();
            table.big_integer("available_at_epoch").not_null();
            table.big_integer("claim_expires_at_epoch").not_null();
            table.timestamps();
        }).await?;

        let pool = Orm::pool()?;
        for statement in [
            "CREATE INDEX quizzes_lesson_status_idx ON quizzes(lesson_id, status)",
            "CREATE UNIQUE INDEX quiz_questions_position_unique ON quiz_questions(quiz_id, position)",
            "CREATE INDEX quiz_questions_enabled_idx ON quiz_questions(quiz_id, enabled, position)",
            "CREATE UNIQUE INDEX quiz_options_position_unique ON quiz_options(question_id, position)",
            "CREATE UNIQUE INDEX quiz_attempts_key_unique ON quiz_attempts(attempt_key)",
            "CREATE INDEX quiz_attempts_subject_idx ON quiz_attempts(subject_user_id, quiz_id, ruleset_version, status)",
            "CREATE UNIQUE INDEX quiz_attempt_sessions_key_unique ON quiz_attempt_sessions(attempt_key)",
            "CREATE INDEX quiz_attempt_sessions_subject_idx ON quiz_attempt_sessions(subject_user_id, quiz_id, ruleset_version, status)",
            "CREATE UNIQUE INDEX quiz_answers_attempt_question_unique ON quiz_answers(attempt_id, question_id)",
            "CREATE INDEX activities_lesson_kind_idx ON activities(lesson_id, activity_kind)",
            "CREATE UNIQUE INDEX activity_attempts_key_unique ON activity_attempts(subject_user_id, activity_id, attempt_key)",
            "CREATE INDEX activity_attempts_subject_idx ON activity_attempts(subject_user_id, activity_id, ruleset_version, finished_at_epoch)",
            "CREATE UNIQUE INDEX achievements_code_unique ON achievements(code)",
            "CREATE UNIQUE INDEX leaderboard_user_course_season_unique ON leaderboard_entries(user_id, course_id, season_key)",
            "CREATE INDEX leaderboard_course_season_score_idx ON leaderboard_entries(course_id, season_key, score)",
            "CREATE INDEX automation_school_trigger_enabled_idx ON automation_rules(school_id, trigger_kind, enabled)",
            "CREATE UNIQUE INDEX user_achievements_school_user_achievement_unique ON user_achievements(school_id, user_id, achievement_id)",
            "CREATE INDEX user_achievements_school_source_idx ON user_achievements(school_id, source_event_key)",
            "CREATE UNIQUE INDEX automation_executions_key_unique ON automation_executions(execution_key)",
            "CREATE INDEX automation_executions_school_source_idx ON automation_executions(school_id, source_event_key, outcome)",
            "CREATE UNIQUE INDEX score_events_idempotency_unique ON score_events(idempotency_key)",
            "CREATE UNIQUE INDEX score_events_attempt_unique ON score_events(origin, subject_user_id, activity_id, attempt_key, ruleset_version)",
            "CREATE INDEX score_events_subject_occurred_idx ON score_events(subject_user_id, occurred_at)",
            "CREATE UNIQUE INDEX score_corrections_key_unique ON score_corrections(correction_key)",
            "CREATE INDEX score_corrections_subject_occurred_idx ON score_corrections(subject_user_id, occurred_at)",
            "CREATE UNIQUE INDEX academy_outbox_event_key_unique ON academy_outbox(event_key)",
            "CREATE INDEX academy_outbox_delivery_idx ON academy_outbox(status, available_at_epoch, claim_expires_at_epoch)",
            "CREATE INDEX academy_outbox_school_delivery_idx ON academy_outbox(school_id, status, available_at_epoch)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?;
        }

        for fixture in [
            "INSERT INTO quizzes (id, lesson_id, activity_id, title, passing_score, max_attempts, time_limit_seconds, ruleset_version, season_key, status) VALUES (1, 1, 2, 'Memory Safety Checkpoint', 80, 3, 0, 'memory-rules-v1', 'season-2026', 'published')",
            "INSERT INTO quizzes (id, lesson_id, activity_id, title, passing_score, max_attempts, time_limit_seconds, ruleset_version, season_key, status) VALUES (2, 1, 3, 'Timed Ownership Checkpoint', 80, 3, 30, 'timed-rules-v1', 'season-2026', 'published')",
            "INSERT INTO quiz_questions (id, quiz_id, prompt, position, points, enabled) VALUES (1, 1, 'Which Rust rule prevents two simultaneous mutable references?', 1, 100, 1)",
            "INSERT INTO quiz_questions (id, quiz_id, prompt, position, points, enabled) VALUES (2, 2, 'Who authorizes a learner-owned attempt?', 1, 100, 1)",
            "INSERT INTO quiz_options (id, question_id, label, position, is_correct) VALUES (1, 1, 'Exclusive mutable borrowing', 1, 1)",
            "INSERT INTO quiz_options (id, question_id, label, position, is_correct) VALUES (2, 1, 'Unchecked shared mutation', 2, 0)",
            "INSERT INTO quiz_options (id, question_id, label, position, is_correct) VALUES (3, 2, 'The authenticated owner or authorized admin', 1, 1)",
            "INSERT INTO quiz_options (id, question_id, label, position, is_correct) VALUES (4, 2, 'Any submitted user identifier', 2, 0)",
            "INSERT INTO activities (id, lesson_id, title, activity_kind, max_score, ruleset_version, season_key, evidence_sha256, config_json) VALUES (1, 1, 'Borrow Checker Rescue', 'game', 100, 'rules-v1', 'season-2026', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', '{\"schema_version\":1,\"mode\":\"offline\"}')",
            "INSERT INTO activities (id, lesson_id, title, activity_kind, max_score, ruleset_version, season_key, evidence_sha256, config_json) VALUES (2, 1, 'Memory Safety Checkpoint', 'quiz', 100, 'memory-rules-v1', 'season-2026', 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', '{\"schema_version\":1,\"ruleset_version\":\"memory-rules-v1\"}')",
            "INSERT INTO activities (id, lesson_id, title, activity_kind, max_score, ruleset_version, season_key, evidence_sha256, config_json) VALUES (3, 1, 'Timed Ownership Checkpoint', 'quiz', 100, 'timed-rules-v1', 'season-2026', 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc', '{\"schema_version\":1,\"ruleset_version\":\"timed-rules-v1\"}')",
            "INSERT INTO activities (id, lesson_id, title, activity_kind, max_score, ruleset_version, season_key, evidence_sha256, config_json) VALUES (4, 1, 'Ownership Listening Choice', 'exercise', 80, 'rules-v1', 'season-2026', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', '{\"schema_version\":1,\"mode\":\"single_choice\",\"correct_option_id\":11}')",
            "INSERT INTO activities (id, lesson_id, title, activity_kind, max_score, ruleset_version, season_key, evidence_sha256, config_json) VALUES (5, 1, 'Ownership Concept Match', 'exercise', 90, 'matching-rules-v1', 'season-2026', 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', '{\"schema_version\":1,\"mode\":\"matching\",\"pairs\":[{\"left_id\":1,\"right_id\":11},{\"left_id\":2,\"right_id\":12},{\"left_id\":3,\"right_id\":13}]}')",
            "INSERT INTO achievements (id, code, name, description, xp_reward, enabled) VALUES (1, 'memory-guardian', 'Memory Guardian', 'Complete the offline memory-safety challenge.', 100, 1)",
            "INSERT INTO automation_rules (id, school_id, name, trigger_kind, action_kind, config_json, enabled) VALUES (1, 1, 'Award Memory Guardian', 'score_recorded', 'award_achievement', '{\"schema_version\":1,\"achievement_code\":\"memory-guardian\",\"minimum_score\":80}', 1)",
            "INSERT INTO automation_rules (id, school_id, name, trigger_kind, action_kind, config_json, enabled) VALUES (2, 2, 'Rival Memory Guardian', 'score_recorded', 'award_achievement', '{\"schema_version\":1,\"achievement_code\":\"memory-guardian\",\"minimum_score\":80}', 1)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(fixture)).execute(pool).await?;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("academy_outbox").await?;
        Schema::drop_if_exists("quiz_answers").await?;
        Schema::drop_if_exists("quiz_attempt_sessions").await?;
        Schema::drop_if_exists("quiz_attempts").await?;
        Schema::drop_if_exists("quiz_options").await?;
        Schema::drop_if_exists("quiz_questions").await?;
        Schema::drop_if_exists("automation_executions").await?;
        Schema::drop_if_exists("user_achievements").await?;
        Schema::drop_if_exists("score_corrections").await?;
        Schema::drop_if_exists("score_events").await?;
        Schema::drop_if_exists("automation_rules").await?;
        Schema::drop_if_exists("leaderboard_entries").await?;
        Schema::drop_if_exists("achievements").await?;
        Schema::drop_if_exists("activity_attempts").await?;
        Schema::drop_if_exists("activities").await?;
        Schema::drop_if_exists("quizzes").await
    }
}
"##;

const MIGRATIONS_MODULE: &str = r##"// Generated by Rullst.
pub mod m20260601000000_create_lms_tables;
pub mod m20260827000000_add_learning_access;
pub mod m20260828000000_add_academy_domains;
pub mod m20260829000000_add_lesson_availability;
pub mod m20260830000000_add_notifications;
pub mod m20260831000000_add_scheduler_leases;
pub mod m20260901000000_add_course_publication;
pub mod m20260901500000_add_school_tenancy;
pub mod m20260902000000_add_education_roles;
pub mod m20260903000000_add_course_completion;
pub mod m20260904000000_add_publication_rollbacks;
pub mod m20260905000000_add_assignments;
pub mod m20260906000000_add_privacy_lifecycle;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260601000000_create_lms_tables::MigrationImpl),
        Box::new(m20260827000000_add_learning_access::MigrationImpl),
        Box::new(m20260828000000_add_academy_domains::MigrationImpl),
        Box::new(m20260829000000_add_lesson_availability::MigrationImpl),
        Box::new(m20260830000000_add_notifications::MigrationImpl),
        Box::new(m20260831000000_add_scheduler_leases::MigrationImpl),
        Box::new(m20260901000000_add_course_publication::MigrationImpl),
        Box::new(m20260901500000_add_school_tenancy::MigrationImpl),
        Box::new(m20260902000000_add_education_roles::MigrationImpl),
        Box::new(m20260903000000_add_course_completion::MigrationImpl),
        Box::new(m20260904000000_add_publication_rollbacks::MigrationImpl),
        Box::new(m20260905000000_add_assignments::MigrationImpl),
        Box::new(m20260906000000_add_privacy_lifecycle::MigrationImpl),
    ]
}

"##;

#[cfg(test)]
mod tests {
    use super::MIGRATION;

    #[test]
    fn academy_schema_has_uniqueness_and_query_indexes() {
        for required in [
            "achievements_code_unique",
            "leaderboard_user_course_season_unique",
            "quizzes_lesson_status_idx",
            "quiz_attempts_key_unique",
            "quiz_attempt_sessions_key_unique",
            "quiz_answers_attempt_question_unique",
            "activity_attempts_key_unique",
            "automation_school_trigger_enabled_idx",
            "user_achievements_school_user_achievement_unique",
            "automation_executions_key_unique",
            "score_events_idempotency_unique",
            "score_events_attempt_unique",
            "score_corrections_key_unique",
            "academy_outbox_event_key_unique",
        ] {
            assert!(MIGRATION.contains(required), "missing index {required}");
        }
        assert!(MIGRATION.contains("Borrow Checker Rescue"));
        assert!(MIGRATION.contains("Memory Guardian"));
        assert!(MIGRATION.contains("activity_attempts(subject_user_id, activity_id, attempt_key)"));
        assert!(!MIGRATION.contains("datetime("));
    }
}
