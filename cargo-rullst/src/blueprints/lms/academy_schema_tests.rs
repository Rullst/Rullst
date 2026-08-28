// Materialized SQLite regression emitted into the generated migrations module.

pub const GENERATED_TESTS_PREFIX: &str = r##"
#[cfg(test)]
mod tests {
    use super::get_migrations;
    use crate::services::assessment_service::{
        AssessmentError, QuizAnswerInput, QuizSubmission, grade_quiz_at,
    };
    use crate::services::assessment_timing_service::{
        QuizStartError, QuizStartRequest, start_quiz_at,
    };
    use crate::services::automation_execution_service::apply_claimed_plan;
    use crate::services::automation_service::{
        AutomationRuleInput, PlannedAction, plan_score_automations,
    };
    use crate::services::automation_worker_service::{
        AutomationWorkerConfig, AutomationWorkerOutcome, run_once_at, start,
    };
    use crate::services::learning_service::{LearningError, authorize_lesson_at, enroll};
    use crate::services::notification_service::{
        NotificationError, list_notifications, mark_read, set_preference, subscribe_in_app,
    };
    use crate::services::notification_template_service::{
        NotificationTemplateError, render_notification,
    };
    use crate::services::outbox_service::{acknowledge, claim_next_at, fail_at};
    use crate::services::progress_service::{ProgressError, correct_progress, record_progress};
    use crate::services::publication_service::{
        PublicationError, create_draft, review_version_at, submit_for_review,
    };
    use crate::services::role_service::{RoleError, active_roles_at, grant_role};
    use crate::services::scheduler_lease_service::{acquire_at, release, renew_at, snapshot};
    use crate::services::score_correction_service::correct_score;
    use crate::services::score_service::{
        SCORE_EVENT_SCHEMA_VERSION, ScoreSubmission, leaderboard, record_score,
    };
    use rullst::db::Orm;
    use rullst_security::UserContext;

    fn tenant_context(user_id: &str, roles: Vec<String>, tenant_key: &str) -> UserContext {
        UserContext::new(user_id, roles)
            .try_with_tenant_id(tenant_key)
            .expect("materialized tenant context")
    }

    fn academy_context(user_id: &str, roles: Vec<String>) -> UserContext {
        tenant_context(user_id, roles, "academy-demo")
    }

    #[tokio::test]
    async fn academy_schema_scores_and_corrections_work_on_sqlite() {
        Orm::init("sqlite:file:rullst_academy_schema?mode=memory&cache=shared")
            .await
            .expect("Academy SQLite should initialize");
        for migration in get_migrations() {
            migration.up().await.expect("Academy migration should run");
        }

        for user_id in 1_i32..=40_i32 {
            rullst::db::sqlx::query(
                "INSERT INTO school_memberships (membership_key, school_id, user_id, status, is_default, valid_from_epoch, expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            )
            .bind(format!("academy-demo-member-{user_id}"))
            .bind(1_i32)
            .bind(user_id)
            .bind("active")
            .bind(1_i32)
            .bind(1_i64)
            .bind(0_i64)
            .execute(Orm::pool().expect("Academy pool"))
            .await
            .expect("Academy school membership fixture");
        }
        rullst::db::sqlx::query(
            "INSERT INTO school_memberships (membership_key, school_id, user_id, status, is_default, valid_from_epoch, expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind("academy-rival-member-41")
        .bind(2_i32)
        .bind(41_i32)
        .bind("active")
        .bind(1_i32)
        .bind(1_i64)
        .bind(0_i64)
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("Rival school membership fixture");

        rullst::db::sqlx::query(
            "INSERT INTO users (id, name, email, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(7_i32)
        .bind("Offline Learner")
        .bind("learner@example.test")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("Academy learner fixture");
        rullst::db::sqlx::query(
            "INSERT INTO enrollments (user_id, course_id, status, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(7_i32)
        .bind(1_i32)
        .bind("active")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("Academy enrollment fixture");

        let fixture_count = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM activities WHERE activity_kind = ?",
        )
        .bind("game")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("Academy fixture query");
        assert_eq!(fixture_count, 1);

        let submission = ScoreSubmission {
            idempotency_key: "event-sqlite-1".to_string(),
            schema_version: SCORE_EVENT_SCHEMA_VERSION,
            origin: "game".to_string(),
            subject_user_id: 7,
            course_id: 1,
            activity_id: 1,
            attempt_key: "attempt-sqlite-1".to_string(),
            points: 80,
            max_score: 100,
            ruleset_version: "rules-v1".to_string(),
            season_key: "season-2026".to_string(),
        };
        let learner = academy_context("7", vec!["student".to_string()]);
        let localized = render_notification(
            "pt-BR",
            "academy.achievement.awarded",
            r#"{"schema_version":1,"achievement_code":"memory-guardian","recorded_actor_user_id":7}"#,
        )
        .expect("Portuguese achievement template");
        assert_eq!(localized.locale, "pt-BR");
        assert_eq!(localized.title, "Nova conquista desbloqueada");
        let fallback = render_notification(
            "de-DE",
            "academy.achievement.awarded",
            r#"{"schema_version":1,"achievement_code":"memory-guardian","recorded_actor_user_id":7}"#,
        )
        .expect("unsupported locale fallback");
        assert_eq!(fallback.locale, "en");
        assert!(matches!(
            render_notification("en", "academy.unknown", "{}"),
            Err(NotificationTemplateError::UnsupportedKey),
        ));
        assert!(record_score(&learner, submission.clone()).await.expect("new score").applied);
        assert!(!record_score(&learner, submission).await.expect("score replay").applied);

        let outbox_count = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM academy_outbox WHERE event_kind = ?",
        )
        .bind("score_recorded")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("Academy outbox query");
        assert_eq!(outbox_count, 1);

        let payload = rullst::db::sqlx::query_scalar::<_, String>(
            "SELECT payload_json FROM academy_outbox WHERE event_key = ?",
        )
        .bind("score:event-sqlite-1")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("Academy outbox payload");
        let (rule_id, enabled, trigger_kind, action_kind, config_json) =
            rullst::db::sqlx::query_as::<_, (i32, i32, String, String, String)>(
                "SELECT id, enabled, trigger_kind, action_kind, config_json FROM automation_rules WHERE id = ?",
            )
            .bind(1_i32)
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("Academy automation fixture");
        let plans = plan_score_automations(
            "score:event-sqlite-1",
            "score_recorded",
            &payload,
            &[AutomationRuleInput {
                id: rule_id,
                enabled: enabled == 1,
                trigger_kind,
                action_kind,
                config_json,
            }],
        )
        .expect("Academy automation dry-run");
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].execution_key, "automation:1:score:event-sqlite-1");
        assert_eq!(
            plans[0].action,
            PlannedAction::AwardAchievement {
                subject_user_id: 7,
                achievement_code: "memory-guardian".to_string(),
            }
        );

        let first_claim = claim_next_at("worker-a", "claim-score-1", 1_000, 30)
            .await
            .expect("first outbox claim")
            .expect("pending event");
        assert_eq!(first_claim.attempts, 1);
        assert!(claim_next_at("worker-b", "claim-score-race", 1_001, 30)
            .await
            .expect("concurrent empty poll")
            .is_none());
        let first_execution = apply_claimed_plan(&plans[0], &first_claim.claim_key)
            .await
            .expect("first automation execution");
        assert!(first_execution.execution_recorded);
        assert!(first_execution.action_applied);
        let mut notification_realtime = subscribe_in_app(&learner, 7)
            .await
            .expect("tenant-scoped notification subscription");
        let notification_outcome = run_once_at(
            "worker-notification",
            "claim-notification",
            1_001,
            30,
            3,
            5,
        )
        .await
        .expect("achievement notification worker");
        assert_eq!(
            notification_outcome,
            AutomationWorkerOutcome::Delivered {
                event_key: "achievement:automation:1:score:event-sqlite-1".to_string(),
                planned_actions: 1,
            }
        );
        let realtime_message = notification_realtime
            .try_recv()
            .expect("tenant-scoped realtime notification");
        assert_eq!(
            realtime_message.channel,
            "tenants:academy-demo:notifications/user/7"
        );
        assert_eq!(realtime_message.event, "notification.created");
        let realtime_payload: serde_json::Value = serde_json::from_str(&realtime_message.payload)
            .expect("realtime notification JSON");
        assert_eq!(realtime_payload["subject_user_id"], 7);
        assert_eq!(realtime_payload["localization_key"], "academy.achievement.awarded");
        assert_eq!(realtime_payload["rendered"]["locale"], "en");
        assert_eq!(
            realtime_payload["rendered"]["title"],
            "New achievement unlocked",
        );
        assert!(fail_at(first_claim.id, &first_claim.claim_key, "offline retry", 2, 1_002, 10)
            .await
            .expect("retry transition"));
        assert!(claim_next_at("worker-b", "claim-score-too-early", 1_011, 30)
            .await
            .expect("backoff poll")
            .is_none());
        let second_claim = claim_next_at("worker-b", "claim-score-2", 1_012, 30)
            .await
            .expect("second outbox claim")
            .expect("retried event");
        assert_eq!(second_claim.attempts, 2);
        let replay_execution = apply_claimed_plan(&plans[0], &second_claim.claim_key)
            .await
            .expect("idempotent automation replay");
        assert!(!replay_execution.execution_recorded);
        assert!(!replay_execution.action_applied);
        assert!(acknowledge(second_claim.id, &second_claim.claim_key)
            .await
            .expect("outbox acknowledgement"));
        let (status, attempts) = rullst::db::sqlx::query_as::<_, (String, i32)>(
            "SELECT status, attempts FROM academy_outbox WHERE id = ?",
        )
        .bind(second_claim.id)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("delivered state");
        assert_eq!(status, "delivered");
        assert_eq!(attempts, 2);
        let achievement_count = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_achievements WHERE school_id = ? AND user_id = ? AND achievement_id = ?",
        )
        .bind(1_i32)
        .bind(7_i32)
        .bind(1_i32)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("awarded achievement query");
        assert_eq!(achievement_count, 1);

        let (notification_id, notification_school_id, notification_status) =
            rullst::db::sqlx::query_as::<_, (i32, i32, String)>(
                "SELECT id, school_id, status FROM notifications WHERE source_event_key = ?",
            )
            .bind("achievement:automation:1:score:event-sqlite-1")
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("in-app achievement notification");
        assert_eq!(notification_school_id, 1);
        assert_eq!(notification_status, "unread");
        let rendered_notifications = list_notifications(&learner, 7, Some("unread"), 0, 10)
            .await
            .expect("localized owner notification list");
        assert_eq!(rendered_notifications.len(), 1);
        assert_eq!(rendered_notifications[0].rendered_locale, "en");
        assert_eq!(rendered_notifications[0].title, "New achievement unlocked");
        assert_eq!(
            rendered_notifications[0].body,
            "You unlocked the memory-guardian achievement.",
        );
        assert!(matches!(
            mark_read(
                &academy_context("8", vec!["student".to_string()]),
                7,
                notification_id,
            )
            .await,
            Err(NotificationError::Forbidden)
        ));
        assert!(mark_read(&learner, 7, notification_id)
            .await
            .expect("notification owner read"));
        assert!(!mark_read(&learner, 7, notification_id)
            .await
            .expect("notification read replay"));

        rullst::db::sqlx::query(
            "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(1_i32)
        .bind("poison:event-1")
        .bind("unsupported_event")
        .bind(7_i32)
        .bind("{\"schema_version\":1}")
        .bind("pending")
        .bind(0_i32)
        .bind("")
        .bind("")
        .bind("")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("poison outbox fixture");
        let poison_first = claim_next_at("worker-a", "claim-poison-1", 2_000, 30)
            .await
            .expect("poison first claim")
            .expect("poison event");
        let reclaimed = claim_next_at("worker-b", "claim-poison-reclaimed", 2_031, 30)
            .await
            .expect("expired claim recovery")
            .expect("abandoned event");
        assert_eq!(reclaimed.id, poison_first.id);
        assert_eq!(reclaimed.attempts, 2);
        assert!(!acknowledge(poison_first.id, &poison_first.claim_key)
            .await
            .expect("stale claim acknowledgement"));
        assert!(fail_at(reclaimed.id, &reclaimed.claim_key, "unsupported event", 3, 2_032, 0)
            .await
            .expect("poison retry"));
        let poison_second = claim_next_at("worker-b", "claim-poison-2", 2_032, 30)
            .await
            .expect("poison second claim")
            .expect("poison retry event");
        assert!(fail_at(poison_second.id, &poison_second.claim_key, "unsupported event", 3, 2_033, 0)
            .await
            .expect("poison dead letter"));
        let (status, attempts, last_error) =
            rullst::db::sqlx::query_as::<_, (String, i32, String)>(
                "SELECT status, attempts, last_error FROM academy_outbox WHERE id = ?",
            )
            .bind(poison_second.id)
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("dead-letter state");
        assert_eq!(status, "dead_letter");
        assert_eq!(attempts, 3);
        assert_eq!(last_error, "unsupported event");

        let entries = leaderboard(&learner, 1, "season-2026", 10)
            .await
            .expect("leaderboard query");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].score, 80);
        assert!(crate::services::score_service::leaderboard_cache_contains(
            &learner,
            1,
            "season-2026",
        )
        .await
        .expect("tenant leaderboard cache state"));

        let admin = academy_context("1", vec!["admin".to_string()]);
        let correction = correct_score(
            &admin,
            "correction-sqlite-1",
            7,
            1,
            "season-2026",
            90,
            "reviewed offline fixture",
            "rules-v1",
        )
        .await
        .expect("score correction");
        assert!(correction.applied);
        assert!(!crate::services::score_service::leaderboard_cache_contains(
            &learner,
            1,
            "season-2026",
        )
        .await
        .expect("score correction invalidates leaderboard cache"));
        let replay = correct_score(
            &admin,
            "correction-sqlite-1",
            7,
            1,
            "season-2026",
            90,
            "reviewed offline fixture",
            "rules-v1",
        )
        .await
        .expect("correction replay");
        assert!(!replay.applied);

        let corrected = leaderboard(&learner, 1, "season-2026", 10)
            .await
            .expect("corrected leaderboard");
        assert_eq!(corrected[0].score, 90);
        assert!(crate::services::score_service::leaderboard_cache_contains(
            &learner,
            1,
            "season-2026",
        )
        .await
        .expect("corrected leaderboard recaches"));

        let first_progress = record_progress(&learner, 7, 1, 40, "progress-sqlite-1")
            .await
            .expect("initial lesson progress");
        assert!(first_progress.applied);
        assert_eq!(first_progress.progress.progress_percent, 40);
        let regression = record_progress(&learner, 7, 1, 25, "progress-sqlite-2")
            .await
            .expect("monotonic lesson progress");
        assert!(!regression.applied);
        assert_eq!(regression.progress.progress_percent, 40);
        let completed = record_progress(&learner, 7, 1, 100, "progress-sqlite-3")
            .await
            .expect("completed lesson progress");
        assert!(completed.applied);
        assert_eq!(completed.progress.completed, 1);
        let completion_replay = record_progress(&learner, 7, 1, 100, "progress-sqlite-3")
            .await
            .expect("progress replay");
        assert!(!completion_replay.applied);
        let completion_events = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM academy_outbox WHERE event_kind = ? AND subject_user_id = ?",
        )
        .bind("lesson_completed")
        .bind(7_i32)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("lesson completion outbox");
        assert_eq!(completion_events, 1);

        assert!(matches!(
            correct_progress(
                &learner,
                "progress-correction-denied",
                7,
                1,
                90,
                "manual review",
            )
            .await,
            Err(ProgressError::Access(_))
        ));
        let progress_correction = correct_progress(
            &admin,
            "progress-correction-sqlite-1",
            7,
            1,
            90,
            "manual review",
        )
        .await
        .expect("admin progress correction");
        assert!(progress_correction.applied);
        assert_eq!(progress_correction.progress.progress_percent, 90);
        assert_eq!(progress_correction.progress.completed, 0);
        let correction_replay = correct_progress(
            &admin,
            "progress-correction-sqlite-1",
            7,
            1,
            90,
            "manual review",
        )
        .await
        .expect("admin progress correction replay");
        assert!(!correction_replay.applied);
        let progress_audit_count = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM lesson_progress_events WHERE subject_user_id = ? AND lesson_id = ?",
        )
        .bind(7_i32)
        .bind(1_i32)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("progress audit history");
        assert_eq!(progress_audit_count, 3);

"##;
