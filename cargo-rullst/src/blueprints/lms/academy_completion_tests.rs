// Final outbox-delivery assertions and closure for the materialized Academy test.

#[path = "academy_rollback_tests.rs"]
mod academy_rollback_tests;
pub(crate) use academy_rollback_tests::GENERATED_ROLLBACK_TESTS_SUFFIX;

pub const GENERATED_COMPLETION_TESTS_SUFFIX: &str = r##"
        let denied_activity = crate::controllers::activity_controller::submit(
            rullst::server::Path(4),
            rullst::server::Extension(8),
            rullst::server::Extension(academy_context("8", vec!["student".to_string()])),
            rullst::server::Json(
                crate::controllers::activity_controller::SingleChoicePayload {
                    attempt_key: "activity-http-denied".to_string(),
                    selected_option_id: 11,
                },
            ),
        )
        .await;
        assert_eq!(denied_activity.status(), rullst::server::StatusCode::FORBIDDEN);
        let activity_payload = || {
            rullst::server::Json(
                crate::controllers::activity_controller::SingleChoicePayload {
                    attempt_key: "activity-http-1".to_string(),
                    selected_option_id: 11,
                },
            )
        };
        let activity_response = crate::controllers::activity_controller::submit(
            rullst::server::Path(4),
            rullst::server::Extension(7),
            rullst::server::Extension(learner.clone()),
            activity_payload(),
        )
        .await;
        assert_eq!(activity_response.status(), rullst::server::StatusCode::OK);
        let activity_replay = crate::controllers::activity_controller::submit(
            rullst::server::Path(4),
            rullst::server::Extension(7),
            rullst::server::Extension(learner.clone()),
            activity_payload(),
        )
        .await;
        assert_eq!(activity_replay.status(), rullst::server::StatusCode::OK);
        let conflicting_activity = crate::controllers::activity_controller::submit(
            rullst::server::Path(4),
            rullst::server::Extension(7),
            rullst::server::Extension(learner.clone()),
            rullst::server::Json(
                crate::controllers::activity_controller::SingleChoicePayload {
                    attempt_key: "activity-http-1".to_string(),
                    selected_option_id: 12,
                },
            ),
        )
        .await;
        assert_eq!(conflicting_activity.status(), rullst::server::StatusCode::CONFLICT);
        let (http_attempts, http_scores, http_events) =
            rullst::db::sqlx::query_as::<_, (i64, i64, i64)>(
                "SELECT (SELECT COUNT(*) FROM activity_attempts WHERE attempt_key = ?), (SELECT COUNT(*) FROM score_events WHERE idempotency_key = ?), (SELECT COUNT(*) FROM academy_outbox WHERE event_key = ?)",
            )
            .bind("activity-http-1")
            .bind("activity:7:4:activity-http-1")
            .bind("score:activity:7:4:activity-http-1")
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("HTTP activity transaction evidence");
        assert_eq!((http_attempts, http_scores, http_events), (1, 1, 1));

        let matching_payload = || {
            rullst::server::Json(
                crate::controllers::activity_matching_controller::MatchingPayload {
                    attempt_key: "matching-http-1".to_string(),
                    pairs: vec![
                        crate::controllers::activity_matching_controller::MatchingPairPayload {
                            left_id: 3,
                            right_id: 13,
                        },
                        crate::controllers::activity_matching_controller::MatchingPairPayload {
                            left_id: 1,
                            right_id: 11,
                        },
                        crate::controllers::activity_matching_controller::MatchingPairPayload {
                            left_id: 2,
                            right_id: 12,
                        },
                    ],
                },
            )
        };
        let matching_response =
            crate::controllers::activity_matching_controller::submit(
                rullst::server::Path(5),
                rullst::server::Extension(7),
                rullst::server::Extension(learner.clone()),
                matching_payload(),
            )
            .await;
        assert_eq!(matching_response.status(), rullst::server::StatusCode::OK);
        let matching_replay = crate::controllers::activity_matching_controller::submit(
            rullst::server::Path(5),
            rullst::server::Extension(7),
            rullst::server::Extension(learner.clone()),
            matching_payload(),
        )
        .await;
        assert_eq!(matching_replay.status(), rullst::server::StatusCode::OK);
        let conflicting_matching =
            crate::controllers::activity_matching_controller::submit(
                rullst::server::Path(5),
                rullst::server::Extension(7),
                rullst::server::Extension(learner.clone()),
                rullst::server::Json(
                    crate::controllers::activity_matching_controller::MatchingPayload {
                        attempt_key: "matching-http-1".to_string(),
                        pairs: vec![
                            crate::controllers::activity_matching_controller::MatchingPairPayload {
                                left_id: 1,
                                right_id: 12,
                            },
                            crate::controllers::activity_matching_controller::MatchingPairPayload {
                                left_id: 2,
                                right_id: 11,
                            },
                            crate::controllers::activity_matching_controller::MatchingPairPayload {
                                left_id: 3,
                                right_id: 13,
                            },
                        ],
                    },
                ),
            )
            .await;
        assert_eq!(
            conflicting_matching.status(),
            rullst::server::StatusCode::CONFLICT
        );
        let invalid_matching = crate::controllers::activity_matching_controller::submit(
            rullst::server::Path(5),
            rullst::server::Extension(7),
            rullst::server::Extension(learner.clone()),
            rullst::server::Json(
                crate::controllers::activity_matching_controller::MatchingPayload {
                    attempt_key: "matching-http-invalid".to_string(),
                    pairs: vec![
                        crate::controllers::activity_matching_controller::MatchingPairPayload {
                            left_id: 1,
                            right_id: 11,
                        },
                        crate::controllers::activity_matching_controller::MatchingPairPayload {
                            left_id: 1,
                            right_id: 12,
                        },
                        crate::controllers::activity_matching_controller::MatchingPairPayload {
                            left_id: 3,
                            right_id: 13,
                        },
                    ],
                },
            ),
        )
        .await;
        assert_eq!(
            invalid_matching.status(),
            rullst::server::StatusCode::UNPROCESSABLE_ENTITY
        );
        let (matching_attempts, matching_scores, matching_events) =
            rullst::db::sqlx::query_as::<_, (i64, i64, i64)>(
                "SELECT (SELECT COUNT(*) FROM activity_attempts WHERE subject_user_id = ? AND activity_id = ? AND attempt_key = ?), (SELECT COUNT(*) FROM score_events WHERE idempotency_key = ?), (SELECT COUNT(*) FROM academy_outbox WHERE event_key = ?)",
            )
            .bind(7_i32)
            .bind(5_i32)
            .bind("matching-http-1")
            .bind("activity:7:5:matching-http-1")
            .bind("score:activity:7:5:matching-http-1")
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("HTTP matching transaction evidence");
        assert_eq!((matching_attempts, matching_scores, matching_events), (1, 1, 1));

        let expected_completion_key = format!(
            "course-completed:22:{}",
            scheduler_draft.id,
        );
        let expected_revocation_key =
            "certificate-revoked:certificate-revoke-22".to_string();
        let mut completion_delivered = false;
        let mut revocation_delivered = false;
        for attempt in 0_i64..20 {
            let pending = rullst::db::sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM academy_outbox WHERE status = ?",
            )
            .bind("pending")
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("pending domain events");
            if pending == 0 {
                break;
            }
            let outcome = run_once_at(
                "worker-completion",
                &format!("claim-completion-{attempt}"),
                80_000 + attempt,
                30,
                3,
                5,
            )
            .await
            .expect("completion journey worker");
            let AutomationWorkerOutcome::Delivered { event_key, .. } = outcome else {
                panic!("completion journey event was not delivered");
            };
            completion_delivered |= event_key == expected_completion_key;
            revocation_delivered |= event_key == expected_revocation_key;
        }
        assert!(completion_delivered);
        assert!(revocation_delivered);
        let remaining = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM academy_outbox WHERE status = ?",
        )
        .bind("pending")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("all domain events delivered");
        assert_eq!(remaining, 0);
    }
}
"##;
