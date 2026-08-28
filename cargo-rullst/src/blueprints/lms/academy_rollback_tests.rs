// Editorial-rollback continuation of the generated materialized Academy test.

pub const GENERATED_ROLLBACK_TESTS_SUFFIX: &str = r##"
        let rollback_admin = academy_context("21", vec!["admin".to_string()]);
        assert!(matches!(
            crate::services::publication_rollback_service::rollback_course_at(
                &academy_context("11", vec!["instructor".to_string()]),
                1, 1, "rollback-course-1-v1-denied", "instructor cannot rollback", 70_020,
            ).await,
            Err(crate::services::publication_rollback_service::PublicationRollbackError::Forbidden)
        ));
        let rollback = crate::services::publication_rollback_service::rollback_course_at(
            &rollback_admin,
            1,
            1,
            "rollback-course-1-v1",
            "restore reviewed stable curriculum after incident",
            70_021,
        )
        .await
        .expect("audited immutable publication rollback");
        assert!(rollback.applied);
        assert_eq!(rollback.source_version_id, 1);
        assert_eq!(rollback.replaced_version_id, scheduler_draft.id);
        assert_eq!(rollback.result_revision, 5);
        assert_eq!(rollback.result_version_key, "rollback:1:rollback-course-1-v1");
        let rollback_replay =
            crate::services::publication_rollback_service::rollback_course_at(
                &rollback_admin,
                1,
                1,
                "rollback-course-1-v1",
                "restore reviewed stable curriculum after incident",
                70_022,
            )
            .await
            .expect("publication rollback replay");
        assert!(!rollback_replay.applied);
        assert_eq!(rollback_replay.result_version_id, rollback.result_version_id);
        assert!(matches!(
            crate::services::publication_rollback_service::rollback_course_at(
                &rollback_admin,
                1,
                http_version_id,
                "rollback-course-1-v1",
                "restore reviewed stable curriculum after incident",
                70_023,
            ).await,
            Err(crate::services::publication_rollback_service::PublicationRollbackError::IdempotencyConflict)
        ));
        let rollback_http = crate::controllers::publication_rollback_controller::rollback(
            rullst::server::Path(1),
            rullst::server::Extension(rollback_admin),
            rullst::server::Json(
                crate::controllers::publication_rollback_controller::PublicationRollbackPayload {
                    source_version_id: 1,
                    rollback_key: "rollback-course-1-v1".to_string(),
                    reason: "restore reviewed stable curriculum after incident".to_string(),
                },
            ),
        )
        .await;
        assert_eq!(rollback_http.status(), rullst::server::StatusCode::OK);

        let rollback_state = rullst::db::sqlx::query_as::<_, (i32, String, String, i32, i64)>(
            "SELECT (SELECT id FROM course_versions WHERE course_id = ? AND status = ?), (SELECT content_json FROM course_versions WHERE id = ?), (SELECT content_json FROM course_versions WHERE id = ?), (SELECT course_version_id FROM enrollment_content_versions WHERE enrollment_id = (SELECT id FROM enrollments WHERE user_id = ? AND course_id = ?)), (SELECT COUNT(*) FROM academy_outbox WHERE event_kind = ? AND event_key = ?)",
        )
        .bind(1_i32)
        .bind("published")
        .bind(rollback.result_version_id)
        .bind(1_i32)
        .bind(22_i32)
        .bind(1_i32)
        .bind("course_rolled_back")
        .bind("course-rolled-back:rollback-course-1-v1")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("publication rollback state");
        assert_eq!(rollback_state.0, rollback.result_version_id);
        assert_eq!(rollback_state.1, rollback_state.2);
        assert_eq!(rollback_state.3, scheduler_draft.id);
        assert_eq!(rollback_state.4, 1);
        let rollback_audit = rullst::db::sqlx::query_as::<_, (i32, i32, i32, i32, String, i64)>(
            "SELECT source_version_id, replaced_version_id, result_version_id, actor_user_id, reason, occurred_at_epoch FROM course_publication_rollbacks WHERE rollback_key = ?",
        )
        .bind("rollback-course-1-v1")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("durable publication rollback audit");
        assert_eq!(rollback_audit.0, 1);
        assert_eq!(rollback_audit.1, scheduler_draft.id);
        assert_eq!(rollback_audit.2, rollback.result_version_id);
        assert_eq!(rollback_audit.3, 21);
        assert_eq!(rollback_audit.4, "restore reviewed stable curriculum after incident");
        assert_eq!(rollback_audit.5, 70_021);

        rullst::db::sqlx::query(
            "INSERT INTO users (id, name, email, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(24_i32)
        .bind("Post Rollback Learner")
        .bind("post-rollback@example.test")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("post rollback learner fixture");
        let post_rollback_learner = academy_context("24", vec!["student".to_string()]);
        let post_rollback_enrollment = enroll(24, &post_rollback_learner, 1)
            .await
            .expect("post rollback enrollment");
        let post_rollback_pin = rullst::db::sqlx::query_scalar::<_, i32>(
            "SELECT course_version_id FROM enrollment_content_versions WHERE enrollment_id = ?",
        )
        .bind(post_rollback_enrollment.id)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("post rollback immutable pin");
        assert_eq!(post_rollback_pin, rollback.result_version_id);
"##;
