// Materialized privacy-lifecycle regression appended to the generated Academy test.

pub const GENERATED_PRIVACY_TESTS_SUFFIX: &str = r##"
        let privacy_admin = academy_context("1", vec!["admin".to_string()]);
        let minor = academy_context("30", vec!["student".to_string()]);
        let guardian = academy_context("31", vec!["guardian".to_string()]);
        let policy = crate::services::privacy_service::configure_subject_policy_at(
            &privacy_admin,
            "privacy-policy-minor-30-v1",
            30,
            crate::services::privacy_service::AgeBand::Minor,
            "academy-privacy-v1",
            200_000,
            100_000,
        )
        .await
        .expect("minor privacy policy");
        assert!(policy.applied);
        assert!(matches!(
            crate::services::privacy_service::authorize_subject_at(
                &minor,
                30,
                "learning",
                100_001,
            )
            .await,
            Err(crate::services::privacy_service::PrivacyError::ConsentRequired)
        ));
        assert!(matches!(
            crate::services::privacy_service::record_guardian_consent_at(
                &tenant_context("41", vec!["guardian".to_string()], "academy-rival"),
                "privacy-rival-consent",
                30,
                "learning",
                "academy-privacy-v1",
                100_002,
            )
            .await,
            Err(crate::services::privacy_service::PrivacyError::Forbidden)
        ));
        let consent = crate::services::privacy_service::record_guardian_consent_at(
            &guardian,
            "privacy-consent-minor-30-learning-v1",
            30,
            "learning",
            "academy-privacy-v1",
            100_003,
        )
        .await
        .expect("same-school guardian consent");
        assert!(consent.applied);
        crate::services::privacy_service::authorize_subject_at(&minor, 30, "learning", 100_004)
            .await
            .expect("active consent authorizes bounded purpose");
        let privacy_request = crate::services::privacy_service::request_privacy_action_at(
            &minor,
            "privacy-export-minor-30-v1",
            30,
            crate::services::privacy_service::PrivacyRequestKind::Export,
            100_005,
        )
        .await
        .expect("owner export request");
        assert!(privacy_request.applied);
        let privacy_replay = crate::services::privacy_service::request_privacy_action_at(
            &minor,
            "privacy-export-minor-30-v1",
            30,
            crate::services::privacy_service::PrivacyRequestKind::Export,
            100_005,
        )
        .await
        .expect("owner export request replay");
        assert!(!privacy_replay.applied);
        assert!(crate::services::privacy_service::revoke_guardian_consent_at(
            &guardian,
            "privacy-consent-minor-30-learning-v1",
            100_006,
        )
        .await
        .expect("guardian consent revocation"));
        assert!(matches!(
            crate::services::privacy_service::authorize_subject_at(
                &minor,
                30,
                "learning",
                100_007,
            )
            .await,
            Err(crate::services::privacy_service::PrivacyError::ConsentRequired)
        ));
        let privacy_rows = rullst::db::sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT (SELECT COUNT(*) FROM privacy_subject_policies WHERE school_id = ? AND subject_user_id = ?), (SELECT COUNT(*) FROM guardian_consents WHERE school_id = ? AND subject_user_id = ?), (SELECT COUNT(*) FROM privacy_requests WHERE school_id = ? AND subject_user_id = ?)",
        )
        .bind(1_i32).bind(30_i32)
        .bind(1_i32).bind(30_i32)
        .bind(1_i32).bind(30_i32)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("school-scoped privacy evidence");
        assert_eq!(privacy_rows, (1, 1, 1));

        crate::services::privacy_service::configure_subject_policy_at(
            &privacy_admin,
            "privacy-policy-adult-29-v1",
            29,
            crate::services::privacy_service::AgeBand::Adult,
            "academy-privacy-v1",
            100_010,
            100_000,
        )
        .await
        .expect("expiring adult privacy policy");
        let rival_retention = crate::services::privacy_retention_service::schedule_expired_at(
            &tenant_context("41", vec!["admin".to_string()], "academy-rival"),
            100_011,
            10,
        )
        .await
        .expect("rival retention sweep remains school scoped");
        assert_eq!(rival_retention.policies_marked, 0);
        assert_eq!(rival_retention.requests_created, 0);
        let retention = crate::services::privacy_retention_service::schedule_expired_at(
            &privacy_admin,
            100_011,
            10,
        )
        .await
        .expect("expired retention sweep");
        assert_eq!(retention.policies_marked, 1);
        assert_eq!(retention.requests_created, 1);
        let retention_replay = crate::services::privacy_retention_service::schedule_expired_at(
            &privacy_admin,
            100_012,
            10,
        )
        .await
        .expect("retention sweep replay");
        assert_eq!(retention_replay.policies_marked, 0);
        assert_eq!(retention_replay.requests_created, 0);
        let retention_state = rullst::db::sqlx::query_as::<_, (String, String, String)>(
            "SELECT p.status, r.request_kind, r.status FROM privacy_subject_policies p INNER JOIN privacy_requests r ON r.school_id = p.school_id AND r.subject_user_id = p.subject_user_id AND r.requested_at_epoch = ? WHERE p.policy_key = ? AND p.school_id = ?",
        )
        .bind(100_011_i64)
        .bind("privacy-policy-adult-29-v1")
        .bind(1_i32)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("durable retention request evidence");
        assert_eq!(
            retention_state,
            (
                "retention_due".to_string(),
                "delete".to_string(),
                "pending".to_string(),
            )
        );

        let rival_claim = crate::services::privacy_request_worker_service::claim_next_at(
            &tenant_context("41", vec!["admin".to_string()], "academy-rival"),
            "privacy-rival-claim",
            100_020,
            5,
        )
        .await
        .expect("rival privacy worker poll");
        assert!(rival_claim.is_none());
        let export_claim = crate::services::privacy_request_worker_service::claim_next_at(
            &privacy_admin,
            "privacy-export-claim-1",
            100_020,
            5,
        )
        .await
        .expect("export privacy request claim")
        .expect("pending export request");
        assert_eq!(export_claim.request_kind, "export");
        assert_eq!(export_claim.subject_user_id, 30);
        assert_eq!(export_claim.attempts, 1);
        let retention_claim = crate::services::privacy_request_worker_service::claim_next_at(
            &privacy_admin,
            "privacy-retention-claim-1",
            100_021,
            5,
        )
        .await
        .expect("retention privacy request claim")
        .expect("pending retention request");
        assert_eq!(retention_claim.request_kind, "delete");
        assert_eq!(retention_claim.subject_user_id, 29);
        let deletion_digest = "a".repeat(64);
        assert!(crate::services::privacy_request_worker_service::complete_at(
            &privacy_admin,
            retention_claim.id,
            &retention_claim.claim_key,
            100_022,
            &deletion_digest,
        )
        .await
        .expect("retention completion"));
        assert!(!crate::services::privacy_request_worker_service::complete_at(
            &privacy_admin,
            export_claim.id,
            "privacy-stale-claim",
            100_024,
            &deletion_digest,
        )
        .await
        .expect("stale completion rejection"));
        assert!(crate::services::privacy_request_worker_service::claim_next_at(
            &privacy_admin,
            "privacy-too-early-recovery",
            100_025,
            5,
        )
        .await
        .expect("privacy lease boundary")
        .is_none());
        let recovered = crate::services::privacy_request_worker_service::claim_next_at(
            &privacy_admin,
            "privacy-export-claim-2",
            100_026,
            5,
        )
        .await
        .expect("expired privacy claim recovery")
        .expect("recovered export request");
        assert_eq!(recovered.id, export_claim.id);
        assert_eq!(recovered.attempts, 2);
        assert!(!crate::services::privacy_request_worker_service::complete_at(
            &privacy_admin,
            export_claim.id,
            &export_claim.claim_key,
            100_027,
            &deletion_digest,
        )
        .await
        .expect("old privacy claim token rejection"));
        assert!(crate::services::privacy_request_worker_service::fail_at(
            &privacy_admin,
            recovered.id,
            &recovered.claim_key,
            "export-adapter-unavailable",
            100_027,
            5,
            3,
        )
        .await
        .expect("privacy export retry"));
        assert!(crate::services::privacy_request_worker_service::claim_next_at(
            &privacy_admin,
            "privacy-retry-too-early",
            100_031,
            5,
        )
        .await
        .expect("privacy retry delay")
        .is_none());
        let final_claim = crate::services::privacy_request_worker_service::claim_next_at(
            &privacy_admin,
            "privacy-export-claim-3",
            100_032,
            5,
        )
        .await
        .expect("privacy final claim")
        .expect("due privacy retry");
        assert_eq!(final_claim.attempts, 3);
        assert!(crate::services::privacy_request_worker_service::fail_at(
            &privacy_admin,
            final_claim.id,
            &final_claim.claim_key,
            "export-adapter-unavailable",
            100_033,
            0,
            3,
        )
        .await
        .expect("privacy dead-letter transition"));
        let privacy_delivery_states = rullst::db::sqlx::query_as::<_, (String, i32, String, String, i32)>(
            "SELECT status, attempts, last_error_code, result_digest, processed_by_user_id FROM privacy_requests WHERE request_key = ?",
        )
        .bind("privacy-export-minor-30-v1")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("privacy export delivery state");
        assert_eq!(
            privacy_delivery_states,
            (
                "dead_letter".to_string(),
                3,
                "export-adapter-unavailable".to_string(),
                "".to_string(),
                1,
            ),
        );
        let completed_retention = rullst::db::sqlx::query_as::<_, (String, String, i32)>(
            "SELECT status, result_digest, processed_by_user_id FROM privacy_requests WHERE id = ?",
        )
        .bind(retention_claim.id)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("completed retention delivery state");
        assert_eq!(completed_retention, ("completed".to_string(), deletion_digest, 1));

        rullst::db::sqlx::query(
            "INSERT INTO privacy_requests (request_key, school_id, subject_user_id, requested_by_user_id, request_kind, status, attempts, claim_key, claim_expires_at_epoch, available_at_epoch, processed_by_user_id, last_error_code, requested_at_epoch, completed_at_epoch, result_digest, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind("privacy-expired-at-hard-limit")
        .bind(1_i32)
        .bind(30_i32)
        .bind(30_i32)
        .bind("export")
        .bind("processing")
        .bind(10_i32)
        .bind("abandoned-hard-limit-claim")
        .bind(100_040_i64)
        .bind(0_i64)
        .bind(0_i32)
        .bind("")
        .bind(100_000_i64)
        .bind(0_i64)
        .bind("")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("hard-limit privacy request fixture");
        assert!(crate::services::privacy_request_worker_service::claim_next_at(
            &privacy_admin,
            "privacy-hard-limit-recovery",
            100_041,
            5,
        )
        .await
        .expect("hard-limit recovery poll")
        .is_none());
        let hard_limit_state = rullst::db::sqlx::query_as::<_, (String, i32, String, i32)>(
            "SELECT status, attempts, last_error_code, processed_by_user_id FROM privacy_requests WHERE request_key = ?",
        )
        .bind("privacy-expired-at-hard-limit")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("hard-limit request state");
        assert_eq!(
            hard_limit_state,
            (
                "dead_letter".to_string(),
                10,
                "claim-expired-at-limit".to_string(),
                1,
            )
        );

        crate::services::privacy_service::request_privacy_action_at(
            &privacy_admin,
            "privacy-supervised-export-29",
            29,
            crate::services::privacy_service::PrivacyRequestKind::Export,
            100_050,
        )
        .await
        .expect("supervised privacy request");
        let executor_digest = "d".repeat(64);
        let privacy_executor =
            crate::services::privacy_request_executor_service::start(
                privacy_admin.clone(),
                crate::services::privacy_request_executor_service::PrivacyExecutorConfig {
                    claim_key_prefix: "privacy-supervised".to_string(),
                    lease_seconds: 30,
                    adapter_timeout_seconds: 1,
                    retry_delay_seconds: 0,
                    max_attempts: 3,
                    idle_delay_millis: 10,
                },
                crate::services::privacy_request_executor_service::DeterministicPrivacyMockAdapter::complete(
                    executor_digest.clone(),
                ),
            )
            .expect("supervised privacy executor start");
        let mut executor_completed = false;
        for _ in 0..300 {
            let status = rullst::db::sqlx::query_scalar::<_, String>(
                "SELECT status FROM privacy_requests WHERE request_key = ?",
            )
            .bind("privacy-supervised-export-29")
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("supervised privacy status");
            if status == "completed" {
                executor_completed = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(executor_completed);
        let executor_metrics = privacy_executor
            .shutdown()
            .await
            .expect("supervised privacy executor shutdown");
        assert_eq!(executor_metrics.completed, 1);
        assert!(executor_metrics.iterations >= 1);
        let executor_state = rullst::db::sqlx::query_as::<_, (String, String, i32)>(
            "SELECT status, result_digest, processed_by_user_id FROM privacy_requests WHERE request_key = ?",
        )
        .bind("privacy-supervised-export-29")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("supervised privacy completion evidence");
        assert_eq!(
            executor_state,
            ("completed".to_string(), executor_digest, 1)
        );

        crate::services::privacy_service::request_privacy_action_at(
            &privacy_admin,
            "privacy-executor-failure-29",
            29,
            crate::services::privacy_service::PrivacyRequestKind::Delete,
            100_060,
        )
        .await
        .expect("privacy executor failure request");
        let executor_failure =
            crate::services::privacy_request_executor_service::run_once_at(
                &privacy_admin,
                &crate::services::privacy_request_executor_service::PrivacyExecutorConfig {
                    claim_key_prefix: "privacy-failure".to_string(),
                    lease_seconds: 30,
                    adapter_timeout_seconds: 1,
                    retry_delay_seconds: 0,
                    max_attempts: 1,
                    idle_delay_millis: 10,
                },
                &crate::services::privacy_request_executor_service::DeterministicPrivacyMockAdapter::fail(
                    "product-policy-unavailable",
                ),
                "privacy-executor-failure-claim",
                100_061,
            )
            .await
            .expect("privacy executor records bounded adapter failure");
        assert_eq!(
            executor_failure,
            crate::services::privacy_request_executor_service::PrivacyExecutorOutcome::Failed {
                request_key: "privacy-executor-failure-29".to_string(),
                error_code: "product-policy-unavailable".to_string(),
                dead_lettered: true,
            }
        );
        let executor_failure_state = rullst::db::sqlx::query_as::<_, (String, i32, String, i32)>(
            "SELECT status, attempts, last_error_code, processed_by_user_id FROM privacy_requests WHERE request_key = ?",
        )
        .bind("privacy-executor-failure-29")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("privacy executor failure evidence");
        assert_eq!(
            executor_failure_state,
            (
                "dead_letter".to_string(),
                1,
                "product-policy-unavailable".to_string(),
                1,
            )
        );
"##;
