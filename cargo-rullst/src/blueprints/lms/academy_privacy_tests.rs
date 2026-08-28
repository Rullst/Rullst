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
"##;
