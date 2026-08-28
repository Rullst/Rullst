// School-isolation continuation of the generated materialized Academy test.

pub const GENERATED_TENANCY_TESTS_SUFFIX: &str = r##"
        let resolved_demo = crate::services::school_service::resolve_membership_at(
            7,
            None,
            90_000,
        )
        .await
        .expect("default school membership");
        assert_eq!(resolved_demo.school_id, 1);
        assert_eq!(resolved_demo.tenant_key, "academy-demo");
        assert!(matches!(
            crate::services::school_service::resolve_membership_at(
                7,
                Some("academy-rival"),
                90_000,
            )
            .await,
            Err(crate::services::school_service::SchoolError::Forbidden)
        ));
        assert!(matches!(
            crate::services::school_service::resolve_membership_at(
                7,
                Some("../academy-demo"),
                90_000,
            )
            .await,
            Err(crate::services::school_service::SchoolError::Forbidden)
        ));

        let rival_context = tenant_context(
            "41",
            vec!["admin".to_string()],
            "academy-rival",
        );
        let invalid_outbox_scopes = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM academy_outbox WHERE school_id <> ?",
        )
        .bind(1_i32)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("tenant-scoped outbox evidence");
        assert_eq!(invalid_outbox_scopes, 0);
        let invalid_projection_scopes = rullst::db::sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT (SELECT COUNT(*) FROM automation_executions WHERE school_id <> ?), (SELECT COUNT(*) FROM user_achievements WHERE school_id <> ?), (SELECT COUNT(*) FROM automation_executions WHERE rule_id = ?)",
        )
        .bind(1_i32)
        .bind(1_i32)
        .bind(2_i32)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("tenant-scoped automation projection evidence");
        assert_eq!(invalid_projection_scopes, (0, 0, 0));
        rullst::db::sqlx::query(
            "INSERT INTO school_memberships (membership_key, school_id, user_id, status, is_default, valid_from_epoch, expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind("academy-rival-member-7")
        .bind(2_i32)
        .bind(7_i32)
        .bind("active")
        .bind(0_i32)
        .bind(1_i64)
        .bind(0_i64)
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("cross-school learner membership fixture");
        let rival_learner = tenant_context(
            "7",
            vec!["student".to_string()],
            "academy-rival",
        );
        assert!(list_notifications(&rival_learner, 7, None, 0, 10)
            .await
            .expect("rival school notification list")
            .is_empty());
        assert!(set_preference(&rival_learner, 7, "in_app", true, "en")
            .await
            .expect("rival school notification preference")
            .applied);
        let scoped_preferences = rullst::db::sqlx::query_as::<_, (i32, i32, String)>(
            "SELECT school_id, enabled, locale FROM notification_preferences WHERE user_id = ? AND channel = ? ORDER BY school_id ASC",
        )
        .bind(7_i32)
        .bind("in_app")
        .fetch_all(Orm::pool().expect("Academy pool"))
        .await
        .expect("school-scoped notification preferences");
        assert_eq!(
            scoped_preferences,
            vec![(1, 0, "pt-BR".to_string()), (2, 1, "en".to_string())],
        );
        assert!(matches!(
            crate::services::school_service::authorize_course(&rival_context, 1).await,
            Err(crate::services::school_service::SchoolError::Forbidden)
        ));
        assert!(matches!(
            crate::services::school_service::authorize_lesson(&rival_context, 1).await,
            Err(crate::services::school_service::SchoolError::Forbidden)
        ));
        assert!(matches!(
            leaderboard(&rival_context, 1, "season-2026", 10).await,
            Err(crate::services::score_service::ScoreError::Forbidden)
        ));
        assert!(matches!(
            create_draft(
                &rival_context,
                1,
                "rival-cross-school-draft",
                "{\"schema_version\":1}",
            )
            .await,
            Err(PublicationError::Forbidden)
        ));
        assert!(matches!(
            crate::services::publication_rollback_service::rollback_course_at(
                &rival_context,
                1,
                1,
                "rival-cross-school-rollback",
                "foreign school rollback must be denied",
                90_000,
            )
            .await,
            Err(crate::services::publication_rollback_service::PublicationRollbackError::Forbidden)
        ));
        assert!(matches!(
            authorize_lesson_at(41, &rival_context, 1, 90_000).await,
            Err(LearningError::Forbidden)
        ));
        let cross_school_http = crate::controllers::learning_controller::enroll(
            rullst::server::Path(1),
            rullst::server::Extension(41),
            rullst::server::Extension(rival_context.clone()),
        )
        .await;
        assert_eq!(
            cross_school_http.status(),
            rullst::server::StatusCode::FORBIDDEN,
        );
        let leaked_enrollments = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM enrollments WHERE user_id = ? AND course_id = ?",
        )
        .bind(41_i32)
        .bind(1_i32)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("cross-school enrollment evidence");
        assert_eq!(leaked_enrollments, 0);
        let foreign_grade = crate::services::assignment_grading_service::AssignmentGradeInput {
            grading_key: "rival-cross-school-grade".to_string(),
            submission_id: assignment_submission.submission_id,
            feedback: "foreign school feedback".to_string(),
            scores: vec![
                crate::services::assignment_grading_service::RubricScoreInput {
                    criterion_id: 1,
                    points_awarded: 50,
                    feedback: "foreign".to_string(),
                },
                crate::services::assignment_grading_service::RubricScoreInput {
                    criterion_id: 2,
                    points_awarded: 30,
                    feedback: "foreign".to_string(),
                },
            ],
        };
        assert!(matches!(
            crate::services::assignment_grading_service::grade_assignment_at(
                &rival_context,
                foreign_grade,
                90_000,
            )
            .await,
            Err(crate::services::assignment_grading_service::AssignmentGradeError::Forbidden)
        ));
        assert!(matches!(
            crate::services::score_correction_service::correct_score(
                &rival_context,
                "rival-cross-school-score",
                7,
                1,
                "season-2026",
                1,
                "foreign school score mutation",
                "rules-v1",
            )
            .await,
            Err(crate::services::score_service::ScoreError::Forbidden)
        ));
        assert!(matches!(
            crate::services::completion_service::revoke_certificate_at(
                &rival_context,
                "rival-cross-school-certificate",
                &completion.certificate_key,
                90_000,
                "foreign school certificate mutation",
            )
            .await,
            Err(crate::services::completion_service::CompletionError::Forbidden)
        ));

        assert!(matches!(
            crate::services::school_service::authorize_course_enrollment_at(
                &rival_context,
                41,
                2,
                90_000,
            )
            .await,
            Err(crate::services::school_service::SchoolError::Forbidden)
        ));
        rullst::db::sqlx::query(
            "INSERT INTO course_entitlements (entitlement_key, school_id, user_id, course_id, source_kind, status, starts_at_epoch, expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind("rival-course-2-user-41")
        .bind(2_i32)
        .bind(41_i32)
        .bind(2_i32)
        .bind("offline_fixture")
        .bind("active")
        .bind(1_i64)
        .bind(0_i64)
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("rival entitlement fixture");
        assert_eq!(
            crate::services::school_service::authorize_course_enrollment_at(
                &rival_context,
                41,
                2,
                90_000,
            )
            .await
            .expect("tenant-scoped entitlement"),
            2,
        );

        rullst::db::sqlx::query(
            "INSERT INTO school_memberships (membership_key, school_id, user_id, status, is_default, valid_from_epoch, expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind("academy-rival-member-40")
        .bind(2_i32)
        .bind(40_i32)
        .bind("active")
        .bind(0_i32)
        .bind(1_i64)
        .bind(0_i64)
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("ambiguous membership fixture");
        rullst::db::sqlx::query(
            "UPDATE school_memberships SET is_default = ? WHERE user_id = ? AND school_id = ?",
        )
        .bind(0_i32)
        .bind(40_i32)
        .bind(1_i32)
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("remove ambiguous default");
        assert!(matches!(
            crate::services::school_service::resolve_membership_at(40, None, 90_000).await,
            Err(crate::services::school_service::SchoolError::AmbiguousMembership)
        ));
        let explicit_rival = crate::services::school_service::resolve_membership_at(
            40,
            Some("academy-rival"),
            90_000,
        )
        .await
        .expect("explicit authenticated school selection");
        assert_eq!(explicit_rival.school_id, 2);

        rullst::db::sqlx::query(
            "INSERT INTO course_versions (course_id, version_key, revision, status, content_json, authored_by, reviewed_by, scheduled_at_epoch, published_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(1_i32)
        .bind("course-1-cross-school-scheduled")
        .bind(99_i32)
        .bind("scheduled")
        .bind("{\"schema_version\":1}")
        .bind(11_i32)
        .bind(12_i32)
        .bind(89_999_i64)
        .bind(0_i64)
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("cross-school scheduler fixture");
        let rival_scheduler = crate::services::publication_scheduler_service::run_cycle_at(
            &rival_context,
            &crate::services::publication_scheduler_service::PublicationSchedulerConfig {
                holder_id: "rival-scheduler".to_string(),
                lease_token_prefix: "rival-cycle".to_string(),
                lease_seconds: 30,
                poll_interval_millis: 10,
                batch_limit: 10,
            },
            "rival-cross-school-cycle",
            90_000,
        )
        .await
        .expect("tenant-scoped scheduler cycle");
        assert_eq!(
            rival_scheduler,
            crate::services::publication_scheduler_service::PublicationSchedulerOutcome::Completed {
                activated: 0,
            },
        );
        let foreign_scheduled_status = rullst::db::sqlx::query_scalar::<_, String>(
            "SELECT status FROM course_versions WHERE version_key = ?",
        )
        .bind("course-1-cross-school-scheduled")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("cross-school scheduler evidence");
        assert_eq!(foreign_scheduled_status, "scheduled");

        assert!(matches!(
            crate::services::role_service::revoke_role_at(
                &rival_context,
                "rival-cross-school-revoke",
                "role-support-7",
                90_000,
                "foreign school must not see the assignment",
            )
            .await,
            Err(crate::services::role_service::RoleError::NotFound)
        ));
"##;
