//! Score/leaderboard schema and materialized SQLite regression.

pub(super) const GAMIFICATION_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m20260828000000_add_gamification" }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("activities", |table| {
            table.id();
            table.integer("lesson_id").not_null();
            table.string("title").not_null();
            table.string("activity_kind").not_null();
            table.integer("max_score").not_null();
            table.integer("max_attempts").not_null();
            table.string("ruleset_version").not_null();
            table.string("status").not_null();
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
            table.string("ruleset_version").not_null();
            table.string("season_key").not_null();
            table.string("evidence_digest").not_null();
            table.big_integer("occurred_at_epoch").not_null();
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

        let pool = Orm::pool()?;
        for statement in [
            "CREATE INDEX activities_lesson_status_idx ON activities(lesson_id, status)",
            "CREATE UNIQUE INDEX score_events_idempotency_unique ON score_events(idempotency_key)",
            "CREATE UNIQUE INDEX score_events_attempt_unique ON score_events(origin, subject_user_id, activity_id, attempt_key, ruleset_version)",
            "CREATE INDEX score_events_subject_idx ON score_events(subject_user_id, activity_id, ruleset_version, occurred_at_epoch)",
            "CREATE UNIQUE INDEX leaderboard_user_course_season_unique ON leaderboard_entries(user_id, course_id, season_key)",
            "CREATE INDEX leaderboard_course_season_score_idx ON leaderboard_entries(course_id, season_key, score)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?;
        }
        sqlx::query(sqlx::AssertSqlSafe(
            "INSERT INTO activities (id, lesson_id, title, activity_kind, max_score, max_attempts, ruleset_version, status) VALUES (1, 1, 'Borrow Checker Rescue', 'game', 100, 3, 'rescue-rules-v1', 'published')",
        )).execute(pool).await?;
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("leaderboard_entries").await?;
        Schema::drop_if_exists("score_events").await?;
        Schema::drop_if_exists("activities").await
    }
}
"##;

pub(super) const GAMIFICATION_MIGRATIONS_MODULE: &str = r##"pub mod m20260601000000_create_lms_tables;
pub mod m20260827000000_add_learning_access;
pub mod m20260828000000_add_gamification;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260601000000_create_lms_tables::MigrationImpl),
        Box::new(m20260827000000_add_learning_access::MigrationImpl),
        Box::new(m20260828000000_add_gamification::MigrationImpl),
    ]
}

#[cfg(test)]
mod tests {
    use super::get_migrations;
    use crate::services::gamification_service::{
        GamificationError, SCORE_EVENT_SCHEMA_VERSION, TrustedActivityResult, leaderboard,
        record_activity_result_at,
    };
    use rullst::db::Orm;
    use rullst_security::UserContext;

    fn result(idempotency_key: impl Into<String>, attempt_key: impl Into<String>, points: i32) -> TrustedActivityResult {
        TrustedActivityResult {
            idempotency_key: idempotency_key.into(),
            schema_version: SCORE_EVENT_SCHEMA_VERSION,
            origin: "game".to_string(),
            subject_user_id: 7,
            activity_id: 1,
            attempt_key: attempt_key.into(),
            points,
            ruleset_version: "rescue-rules-v1".to_string(),
            season_key: "season-2026".to_string(),
            evidence_digest: "a".repeat(64),
        }
    }

    #[tokio::test]
    async fn score_is_trusted_idempotent_attempt_bounded_and_transactional() {
        Orm::init("sqlite:file:rullst_detached_gamification?mode=memory&cache=shared")
            .await.expect("gamification SQLite should initialize");
        for migration in get_migrations() {
            migration.up().await.expect("gamification migration should run");
        }
        rullst::db::sqlx::query(
            "INSERT INTO users (id, name, email, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        ).bind(7_i32).bind("Offline Player").bind("player@example.test")
            .execute(Orm::pool().expect("gamification pool")).await
            .expect("gamification player fixture");
        rullst::db::sqlx::query(
            "INSERT INTO enrollments (user_id, course_id, status, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        ).bind(7_i32).bind(1_i32).bind("active")
            .execute(Orm::pool().expect("gamification pool")).await
            .expect("gamification enrollment fixture");

        let player = UserContext::new("7", vec!["student".to_string()]);
        let first = result("game-event-1", "game-attempt-1", 80);
        assert!(record_activity_result_at(&player, first.clone(), 5_000).await
            .expect("first trusted score").applied);
        assert!(!record_activity_result_at(&player, first.clone(), 5_001).await
            .expect("score replay").applied);

        let mut changed = first.clone();
        changed.points = 90;
        assert!(matches!(
            record_activity_result_at(&player, changed, 5_002).await,
            Err(GamificationError::IdempotencyConflict)
        ));
        let duplicate_attempt = result("game-event-other", "game-attempt-1", 80);
        assert!(matches!(
            record_activity_result_at(&player, duplicate_attempt, 5_003).await,
            Err(GamificationError::IdempotencyConflict)
        ));
        assert!(matches!(
            record_activity_result_at(
                &UserContext::new("8", vec!["student".to_string()]),
                first,
                5_004,
            ).await,
            Err(GamificationError::Access(_))
        ));
        assert!(matches!(
            record_activity_result_at(&player, result("game-impossible", "game-impossible", 101), 5_005).await,
            Err(GamificationError::InvalidField("score bounds"))
        ));

        for (number, points) in [(2, 20), (3, 0)] {
            assert!(record_activity_result_at(
                &player,
                result(format!("game-event-{number}"), format!("game-attempt-{number}"), points),
                5_000 + i64::from(number),
            ).await.expect("bounded trusted score").applied);
        }
        assert!(matches!(
            record_activity_result_at(&player, result("game-event-4", "game-attempt-4", 10), 5_010).await,
            Err(GamificationError::AttemptLimit)
        ));
        let ranking = leaderboard(&player, 7, 1, "season-2026", 25).await
            .expect("authoritative leaderboard");
        assert_eq!(ranking.len(), 1);
        assert_eq!(ranking[0].score, 100);
        let counts = rullst::db::sqlx::query_as::<_, (i64, i64)>(
            "SELECT (SELECT COUNT(*) FROM score_events), (SELECT COUNT(*) FROM leaderboard_entries)",
        ).fetch_one(Orm::pool().expect("gamification pool")).await
            .expect("gamification persistence counts");
        assert_eq!(counts, (3, 1));
    }
}
"##;
