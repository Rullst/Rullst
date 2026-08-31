// Timed-assessment continuation of the generated materialized SQLite test.

pub const GENERATED_TESTS_SUFFIX: &str = r##"
        let timed_without_start = QuizSubmission {
            attempt_key: "timed-without-start".to_string(),
            quiz_id: 2,
            subject_user_id: 7,
            ruleset_version: "timed-rules-v1".to_string(),
            answers: vec![QuizAnswerInput { question_id: 2, option_id: 3 }],
        };
        assert!(matches!(
            grade_quiz_at(&learner, timed_without_start, 4_000).await,
            Err(AssessmentError::AttemptNotStarted)
        ));

        let expired_start = QuizStartRequest {
            attempt_key: "timed-expired".to_string(),
            quiz_id: 2,
            subject_user_id: 7,
            ruleset_version: "timed-rules-v1".to_string(),
        };
        let started = start_quiz_at(&learner, expired_start.clone(), 4_000)
            .await
            .expect("timed quiz start");
        assert!(started.applied);
        assert_eq!((started.started_at_epoch, started.expires_at_epoch), (4_000, 4_030));
        assert_eq!(started.presentation.question_ids, vec![2]);
        assert_eq!(started.presentation.option_orders.len(), 1);
        assert_eq!(started.presentation.option_orders[0].option_ids.len(), 2);
        let start_replay = start_quiz_at(&learner, expired_start.clone(), 4_001)
            .await
            .expect("timed start replay");
        assert!(!start_replay.applied);
        assert_eq!(start_replay.expires_at_epoch, 4_030);
        assert_eq!(start_replay.presentation, started.presentation);
        assert!(matches!(
            start_quiz_at(
                &academy_context("8", vec!["student".to_string()]),
                expired_start,
                4_002,
            )
            .await,
            Err(QuizStartError::Access(_))
        ));
        let expired_submission = QuizSubmission {
            attempt_key: "timed-expired".to_string(),
            quiz_id: 2,
            subject_user_id: 7,
            ruleset_version: "timed-rules-v1".to_string(),
            answers: vec![QuizAnswerInput { question_id: 2, option_id: 3 }],
        };
        assert!(matches!(
            grade_quiz_at(&learner, expired_submission, 4_031).await,
            Err(AssessmentError::AttemptExpired)
        ));

        let valid_start = QuizStartRequest {
            attempt_key: "timed-valid".to_string(),
            quiz_id: 2,
            subject_user_id: 7,
            ruleset_version: "timed-rules-v1".to_string(),
        };
        start_quiz_at(&learner, valid_start, 5_000)
            .await
            .expect("valid timed start");
        let valid_submission = QuizSubmission {
            attempt_key: "timed-valid".to_string(),
            quiz_id: 2,
            subject_user_id: 7,
            ruleset_version: "timed-rules-v1".to_string(),
            answers: vec![QuizAnswerInput { question_id: 2, option_id: 3 }],
        };
        let timed_grade = grade_quiz_at(&learner, valid_submission.clone(), 5_029)
            .await
            .expect("grade inside server window");
        assert!(timed_grade.applied);
        assert_eq!(timed_grade.score_percent, 100);
        assert!(!grade_quiz_at(&learner, valid_submission, 5_031)
            .await
            .expect("graded replay after expiry")
            .applied);

        start_quiz_at(
            &learner,
            QuizStartRequest {
                attempt_key: "timed-consumed".to_string(),
                quiz_id: 2,
                subject_user_id: 7,
                ruleset_version: "timed-rules-v1".to_string(),
            },
            6_000,
        )
        .await
        .expect("third timed attempt is consumed at start");
        assert!(matches!(
            start_quiz_at(
                &learner,
                QuizStartRequest {
                    attempt_key: "timed-over-limit".to_string(),
                    quiz_id: 2,
                    subject_user_id: 7,
                    ruleset_version: "timed-rules-v1".to_string(),
                },
                7_000,
            )
            .await,
            Err(QuizStartError::AttemptLimit)
        ));
        let (sessions, graded_sessions) =
            rullst::db::sqlx::query_as::<_, (i64, i64)>(
                "SELECT COUNT(*), SUM(CASE WHEN status = 'graded' THEN 1 ELSE 0 END) FROM quiz_attempt_sessions",
            )
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("timed session states");
        assert_eq!((sessions, graded_sessions), (3, 1));
        let timed_ranked = leaderboard(&learner, 1, "season-2026", 10)
            .await
            .expect("timed quiz leaderboard");
        assert_eq!(timed_ranked[0].score, 290);

        assert!(matches!(
            authorize_lesson_at(7, &learner, 2, 8_000).await,
            Err(LearningError::PrerequisiteNotMet)
        ));
        let prerequisite_completion =
            record_progress(&learner, 7, 1, 100, "progress-recompleted")
                .await
                .expect("recomplete corrected prerequisite");
        assert!(prerequisite_completion.applied);
        assert!(authorize_lesson_at(7, &learner, 2, 8_000).await.is_ok());
        rullst::db::sqlx::query(
            "UPDATE lesson_release_rules SET release_at_epoch = ?, expire_at_epoch = ? WHERE lesson_id = ? AND ruleset_version = ?",
        )
        .bind(9_000_i64)
        .bind(9_100_i64)
        .bind(2_i32)
        .bind("lesson-2-v1")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("dated release policy");
        assert!(matches!(
            authorize_lesson_at(7, &learner, 2, 8_999).await,
            Err(LearningError::NotReleased)
        ));
        assert!(authorize_lesson_at(7, &learner, 2, 9_050).await.is_ok());
        assert!(matches!(
            authorize_lesson_at(7, &learner, 2, 9_101).await,
            Err(LearningError::Expired)
        ));
        rullst::db::sqlx::query(
            "INSERT INTO lesson_release_rules (lesson_id, ruleset_version, release_at_epoch, expire_at_epoch, prerequisite_lesson_id, required_progress_percent, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(2_i32)
        .bind("lesson-2-conflicting-v2")
        .bind(9_000_i64)
        .bind(9_100_i64)
        .bind(1_i32)
        .bind(100_i32)
        .bind("active")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("conflicting active policy fixture");
        assert!(matches!(
            authorize_lesson_at(7, &learner, 2, 9_050).await,
            Err(LearningError::InvalidAvailabilityPolicy)
        ));

        rullst::db::sqlx::query(
            "UPDATE academy_outbox SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE status = ?",
        )
        .bind("delivered")
        .bind("pending")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("isolate worker fixture");
        let worker_score = evaluate_activity(
            &learner,
            ActivityAttempt {
                schema_version: ACTIVITY_SCHEMA_VERSION,
                attempt_key: "worker-score".to_string(),
                activity_id: 4,
                subject_user_id: 7,
                kind: ActivityKind::Exercise,
                ruleset_version: "rules-v1".to_string(),
                started_at_epoch_seconds: 11_000,
                state_json: "{\"prompt_version\":1}".to_string(),
            },
            &SingleChoiceSubmission {
                selected_option_id: 11,
            },
            11_030,
            &evaluator,
        )
        .expect("worker authoritative activity outcome");
        assert!(record_activity_result(&learner, worker_score)
            .await
            .expect("worker score fixture")
            .applied);
        let worker_outcome = run_once_at("worker-live", "worker-claim", 12_000, 30, 3, 5)
            .await
            .expect("operable automation worker");
        assert_eq!(
            worker_outcome,
            AutomationWorkerOutcome::Delivered {
                event_key: "score:worker-score".to_string(),
                planned_actions: 1,
            }
        );
        let (delivered, execution_count) = rullst::db::sqlx::query_as::<_, (String, i64)>(
            "SELECT status, (SELECT COUNT(*) FROM automation_executions WHERE source_event_key = ?) FROM academy_outbox WHERE event_key = ?",
        )
        .bind("score:worker-score")
        .bind("score:worker-score")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("worker delivery state");
        assert_eq!(delivered, "delivered");
        assert_eq!(execution_count, 1);
        let worker_ranked = leaderboard(&learner, 1, "season-2026", 10)
            .await
            .expect("worker score leaderboard");
        assert_eq!(worker_ranked[0].score, 370);

        assert!(acquire_at("academy-scheduler", "instance-a", "lease-a", 20_000, 30)
            .await
            .expect("first scheduler lease"));
        assert!(!acquire_at("academy-scheduler", "instance-b", "lease-b", 20_001, 30)
            .await
            .expect("competing scheduler lease"));
        assert!(!renew_at("academy-scheduler", "instance-a", "wrong-token", 20_010, 30)
            .await
            .expect("wrong-token renewal"));
        assert!(renew_at("academy-scheduler", "instance-a", "lease-a", 20_010, 30)
            .await
            .expect("exact scheduler renewal"));
        assert!(!acquire_at("academy-scheduler", "instance-b", "lease-b", 20_039, 30)
            .await
            .expect("pre-expiry takeover"));
        assert!(acquire_at("academy-scheduler", "instance-b", "lease-b", 20_040, 30)
            .await
            .expect("expired scheduler takeover"));
        assert!(!release("academy-scheduler", "instance-a", "lease-a")
            .await
            .expect("stale scheduler release"));
        let held = snapshot("academy-scheduler")
            .await
            .expect("scheduler snapshot")
            .expect("scheduler lease row");
        assert_eq!(held.holder_id, "instance-b");
        assert_eq!(held.expires_at_epoch, 20_070);
        assert!(release("academy-scheduler", "instance-b", "lease-b")
            .await
            .expect("exact scheduler release"));

        let read_notifications = list_notifications(&learner, 7, Some("read"), 0, 10)
            .await
            .expect("owner notification listing");
        assert_eq!(read_notifications.len(), 1);
        assert_eq!(read_notifications[0].status, "read");
        assert!(matches!(
            list_notifications(
                &academy_context("8", vec!["student".to_string()]),
                7,
                None,
                0,
                10,
            )
            .await,
            Err(NotificationError::Forbidden)
        ));
        let disabled = set_preference(&learner, 7, "in_app", false, "pt-BR")
            .await
            .expect("owner notification preference");
        assert!(disabled.applied);
        assert!(!disabled.enabled);
        assert!(!set_preference(&learner, 7, "in_app", false, "pt-BR")
            .await
            .expect("preference replay")
            .applied);
        assert!(matches!(
            set_preference(
                &academy_context("8", vec!["student".to_string()]),
                7,
                "in_app",
                true,
                "en",
            )
            .await,
            Err(NotificationError::Forbidden)
        ));

        rullst::db::sqlx::query(
            "INSERT INTO users (id, name, email, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(9_i32)
        .bind("Enrollment Learner")
        .bind("enrollment@example.test")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("enrollment learner fixture");
        let enrollment_context = academy_context("9", vec!["student".to_string()]);
        let first_enrollment = enroll(9, &enrollment_context, 1)
            .await
            .expect("transactional enrollment");
        let enrollment_replay = enroll(9, &enrollment_context, 1)
            .await
            .expect("idempotent enrollment replay");
        assert_eq!(first_enrollment.id, enrollment_replay.id);
        let enrollment_event_count = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM academy_outbox WHERE event_key = ? AND event_kind = ? AND status = ?",
        )
        .bind("enrollment:9:1")
        .bind("enrollment_activated")
        .bind("pending")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("transactional enrollment event");
        assert_eq!(enrollment_event_count, 1);
        assert_eq!(
            run_once_at("worker-enrollment", "claim-enrollment", 30_000, 30, 3, 5)
                .await
                .expect("enrollment event worker"),
            AutomationWorkerOutcome::Delivered {
                event_key: "enrollment:9:1".to_string(),
                planned_actions: 0,
            }
        );

        rullst::db::sqlx::query(
            "INSERT INTO users (id, name, email, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(10_i32)
        .bind("Supervised Worker Learner")
        .bind("worker@example.test")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("supervised worker learner fixture");
        let supervised_context = academy_context("10", vec!["student".to_string()]);
        enroll(10, &supervised_context, 1)
            .await
            .expect("supervised worker enrollment");
        let worker = start(AutomationWorkerConfig {
            worker_id: "worker-supervised".to_string(),
            claim_key_prefix: "claim-supervised".to_string(),
            lease_seconds: 30,
            max_attempts: 3,
            retry_delay_seconds: 5,
            idle_delay_millis: 5,
        })
        .expect("supervised worker start");
        let mut supervised_delivered = false;
        for _ in 0..100 {
            let status = rullst::db::sqlx::query_scalar::<_, String>(
                "SELECT status FROM academy_outbox WHERE event_key = ?",
            )
            .bind("enrollment:10:1")
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("supervised delivery state");
            if status == "delivered" {
                supervised_delivered = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(supervised_delivered);
        let worker_metrics = worker.shutdown().await.expect("supervised worker shutdown");
        assert_eq!(worker_metrics.delivered, 1);
        assert!(worker_metrics.iterations >= 1);

        let instructor = academy_context("11", vec!["instructor".to_string()]);
        let reviewer = academy_context("12", vec!["admin".to_string()]);
        let draft = create_draft(
            &instructor,
            1,
            "course-1-v2",
            "{\"schema_version\":1,\"lesson_ids\":[1,2],\"release\":\"v2\",\"completion\":{\"schema_version\":1,\"ruleset_version\":\"course-1-completion-v2\",\"required_lesson_ids\":[1,2],\"required_progress_percent\":100}}",
        )
        .await
        .expect("versioned course draft");
        assert_eq!(draft.status, "draft");
        assert!(submit_for_review(&instructor, draft.id)
            .await
            .expect("author submits own version"));
        assert!(matches!(
            review_version_at(&instructor, draft.id, 40_000, 35_000).await,
            Err(PublicationError::Forbidden)
        ));
        let scheduled = review_version_at(&reviewer, draft.id, 40_000, 35_000)
            .await
            .expect("independent reviewer schedules version");
        assert_eq!(scheduled.status, "scheduled");

        rullst::db::sqlx::query(
            "INSERT INTO users (id, name, email, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(13_i32)
        .bind("Pinned Before Publication")
        .bind("pinned-before@example.test")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("pre-publication learner");
        let before_context = academy_context("13", vec!["student".to_string()]);
        let before_enrollment = enroll(13, &before_context, 1)
            .await
            .expect("enrollment before publication");
        let published = review_version_at(&reviewer, draft.id, 40_000, 40_000)
            .await
            .expect("scheduled publication activation");
        assert_eq!(published.status, "published");
        rullst::db::sqlx::query(
            "INSERT INTO users (id, name, email, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(14_i32)
        .bind("Pinned After Publication")
        .bind("pinned-after@example.test")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("post-publication learner");
        let after_context = academy_context("14", vec!["student".to_string()]);
        let after_enrollment = enroll(14, &after_context, 1)
            .await
            .expect("enrollment after publication");
        let (before_pin, after_pin) = rullst::db::sqlx::query_as::<_, (i32, i32)>(
            "SELECT (SELECT course_version_id FROM enrollment_content_versions WHERE enrollment_id = ?), (SELECT course_version_id FROM enrollment_content_versions WHERE enrollment_id = ?)",
        )
        .bind(before_enrollment.id)
        .bind(after_enrollment.id)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("immutable enrollment content pins");
        assert_eq!(before_pin, 1);
        assert_eq!(after_pin, draft.id);
        let mut publication_delivery_order = Vec::new();
        for (claim_key, observed_at) in [
            ("claim-publication-1", 41_000_i64),
            ("claim-publication-2", 41_001_i64),
            ("claim-publication-3", 41_002_i64),
        ] {
            let outcome = run_once_at("worker-publication", claim_key, observed_at, 30, 3, 5)
                .await
                .expect("publication journey worker");
            let AutomationWorkerOutcome::Delivered { event_key, planned_actions } = outcome else {
                panic!("publication journey event was not delivered");
            };
            assert_eq!(planned_actions, 0);
            publication_delivery_order.push(event_key);
        }
        assert_eq!(
            publication_delivery_order,
            vec![
                "enrollment:13:1".to_string(),
                "course-published:course-1-v2".to_string(),
                "enrollment:14:1".to_string(),
            ]
        );

        let school_owner = academy_context("20", vec!["school_owner".to_string()]);
        let temporary = grant_role(
            &school_owner,
            "role-support-7",
            7,
            "support",
            50_000,
            50_100,
            "bounded incident assistance",
        )
        .await
        .expect("temporary support role");
        assert!(temporary.applied);
        assert!(!grant_role(
            &school_owner, "role-support-7", 7, "support", 50_000, 50_100,
            "bounded incident assistance",
        ).await.expect("role grant replay").applied);
        assert_eq!(
            active_roles_at(&learner, 7, 50_050).await.expect("active roles"),
            vec!["support".to_string()]
        );
        assert_eq!(
            active_roles_at(&learner, 7, 50_100).await.expect("expired roles"),
            vec!["student".to_string()]
        );
        assert!(matches!(
            grant_role(&reviewer, "role-admin-15", 15, "admin", 50_000, 0, "owner approval required").await,
            Err(RoleError::Forbidden)
        ));
        assert!(matches!(
            grant_role(&school_owner, "role-support-7", 7, "support", 50_000, 50_200, "bounded incident assistance").await,
            Err(RoleError::IdempotencyConflict)
        ));
"##;
