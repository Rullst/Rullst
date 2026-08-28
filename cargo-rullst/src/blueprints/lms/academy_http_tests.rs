// HTTP-boundary continuation of the generated materialized Academy test.

#[path = "academy_assignment_tests.rs"]
mod academy_assignment_tests;
#[path = "academy_completion_tests.rs"]
pub(super) mod academy_completion_tests;
pub(super) use academy_assignment_tests::GENERATED_ASSIGNMENT_TESTS_SUFFIX;
pub(super) use academy_completion_tests::GENERATED_ROLLBACK_TESTS_SUFFIX;

pub const GENERATED_HTTP_TESTS_SUFFIX: &str = r##"
        let draft_response = crate::controllers::publication_controller::draft(
            rullst::server::Path(1),
            rullst::server::Extension(instructor.clone()),
            rullst::server::Json(
                crate::controllers::publication_controller::DraftPayload {
                    version_key: "course-1-v3-http".to_string(),
                    content: serde_json::json!({
                        "schema_version": 1,
                        "lesson_ids": [1, 2],
                        "release": "v3-http",
                        "completion": {
                            "schema_version": 1,
                            "ruleset_version": "course-1-completion-v3",
                            "required_lesson_ids": [1, 2],
                            "required_progress_percent": 100
                        }
                    }),
                },
            ),
        )
        .await;
        assert_eq!(draft_response.status(), rullst::server::StatusCode::CREATED);
        let http_version_id = rullst::db::sqlx::query_scalar::<_, i32>(
            "SELECT id FROM course_versions WHERE version_key = ?",
        )
        .bind("course-1-v3-http")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("HTTP-created version");
        let submit_response = crate::controllers::publication_controller::submit(
            rullst::server::Path(http_version_id),
            rullst::server::Extension(instructor),
        )
        .await;
        assert_eq!(submit_response.status(), rullst::server::StatusCode::NO_CONTENT);
        let review_response = crate::controllers::publication_controller::review(
            rullst::server::Path(http_version_id),
            rullst::server::Extension(reviewer),
            rullst::server::Json(
                crate::controllers::publication_controller::ReviewPayload {
                    activate_at_epoch: 0,
                },
            ),
        )
        .await;
        assert_eq!(review_response.status(), rullst::server::StatusCode::OK);
        let http_status = rullst::db::sqlx::query_scalar::<_, String>(
            "SELECT status FROM course_versions WHERE id = ?",
        )
        .bind(http_version_id)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("HTTP-reviewed version");
        assert_eq!(http_status, "published");

        let revoked = crate::services::role_service::revoke_role_at(
            &school_owner,
            "revoke-support-7",
            "role-support-7",
            50_060,
            "incident assistance completed",
        )
        .await
        .expect("audited support revocation");
        assert!(revoked.applied);
        assert!(!crate::services::role_service::revoke_role_at(
            &school_owner,
            "revoke-support-7",
            "role-support-7",
            50_080,
            "incident assistance completed",
        )
        .await
        .expect("revocation replay")
        .applied);
        assert_eq!(
            active_roles_at(&learner, 7, 50_070).await.expect("roles after revoke"),
            vec!["student".to_string()]
        );
        let revocation_audit = rullst::db::sqlx::query_as::<_, (String, i32, i64, String)>(
            "SELECT revocation_key, revoked_by, revoked_at_epoch, revocation_reason FROM role_assignments WHERE assignment_key = ?",
        )
        .bind("role-support-7")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("durable revocation audit");
        assert_eq!(
            revocation_audit,
            (
                "revoke-support-7".to_string(),
                20,
                50_060,
                "incident assistance completed".to_string(),
            )
        );
        assert!(matches!(
            crate::services::role_service::revoke_role_at(
                &school_owner, "different-revoke", "role-support-7", 50_090,
                "incident assistance completed",
            ).await,
            Err(RoleError::IdempotencyConflict)
        ));
        grant_role(
            &school_owner, "role-admin-15", 15, "admin", 50_000, 0,
            "school owner approved administrator",
        ).await.expect("owner grants privileged role");
        assert!(matches!(
            crate::services::role_service::revoke_role_at(
                &academy_context("12", vec!["admin".to_string()]),
                "revoke-admin-15", "role-admin-15", 50_090, "admin cannot revoke peer admin",
            ).await,
            Err(RoleError::Forbidden)
        ));
        let role_grant_response = crate::controllers::role_controller::grant(
            rullst::server::Path(16),
            rullst::server::Extension(school_owner.clone()),
            rullst::server::Json(crate::controllers::role_controller::GrantPayload {
                assignment_key: "role-support-16-http".to_string(),
                role: "support".to_string(),
                valid_from_epoch: 60_000,
                expires_at_epoch: 60_100,
                reason: "HTTP support elevation".to_string(),
            }),
        )
        .await;
        assert_eq!(role_grant_response.status(), rullst::server::StatusCode::CREATED);
        let role_revoke_response = crate::controllers::role_controller::revoke(
            rullst::server::Path("role-support-16-http".to_string()),
            rullst::server::Extension(school_owner),
            rullst::server::Json(crate::controllers::role_controller::RevokePayload {
                revocation_key: "revoke-support-16-http".to_string(),
                reason: "HTTP support elevation closed".to_string(),
            }),
        )
        .await;
        assert_eq!(role_revoke_response.status(), rullst::server::StatusCode::OK);
        let http_role_status = rullst::db::sqlx::query_scalar::<_, String>(
            "SELECT status FROM role_assignments WHERE assignment_key = ?",
        )
        .bind("role-support-16-http")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("HTTP role lifecycle");
        assert_eq!(http_role_status, "revoked");

        let scheduler_now = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("scheduler clock")
                .as_secs(),
        )
        .expect("scheduler epoch fits i64");
        let scheduler_author = academy_context("11", vec!["instructor".to_string()]);
        let scheduler_reviewer = academy_context("12", vec!["admin".to_string()]);
        let scheduler_actor = academy_context("21", vec!["admin".to_string()]);
        let scheduler_draft = create_draft(
            &scheduler_author,
            1,
            "course-1-v4-scheduler",
            "{\"schema_version\":1,\"lesson_ids\":[1,2],\"release\":\"v4-scheduler\",\"completion\":{\"schema_version\":1,\"ruleset_version\":\"course-1-completion-v4\",\"required_lesson_ids\":[1,2],\"required_progress_percent\":100}}",
        )
        .await
        .expect("scheduled activation draft");
        assert!(submit_for_review(&scheduler_author, scheduler_draft.id)
            .await
            .expect("scheduled activation submission"));
        let scheduled_for_worker = review_version_at(
            &scheduler_reviewer,
            scheduler_draft.id,
            scheduler_now + 1,
            scheduler_now,
        )
        .await
        .expect("independent scheduled review");
        assert_eq!(scheduled_for_worker.status, "scheduled");

        let scheduler_config = crate::services::publication_scheduler_service::PublicationSchedulerConfig {
            holder_id: "publication-scheduler-a".to_string(),
            lease_token_prefix: "publication-cycle".to_string(),
            lease_seconds: 30,
            poll_interval_millis: 10,
            batch_limit: 10,
        };
        assert!(acquire_at(
            crate::services::publication_scheduler_service::PUBLICATION_LEASE_KEY,
            "publication-scheduler-blocker",
            "publication-blocker-token",
            scheduler_now,
            30,
        )
        .await
        .expect("scheduler blocker lease"));
        let standby = crate::services::publication_scheduler_service::run_cycle_at(
            &scheduler_actor,
            &scheduler_config,
            "publication-standby-token",
            scheduler_now,
        )
        .await
        .expect("contended scheduler cycle");
        assert_eq!(
            standby,
            crate::services::publication_scheduler_service::PublicationSchedulerOutcome::Standby
        );
        assert!(release(
            crate::services::publication_scheduler_service::PUBLICATION_LEASE_KEY,
            "publication-scheduler-blocker",
            "publication-blocker-token",
        )
        .await
        .expect("release scheduler blocker"));

        let scheduler = crate::services::publication_scheduler_service::start(
            scheduler_actor.clone(),
            scheduler_config.clone(),
        )
        .expect("supervised publication scheduler start");
        let mut scheduler_published = false;
        for _ in 0..300 {
            let status = rullst::db::sqlx::query_scalar::<_, String>(
                "SELECT status FROM course_versions WHERE id = ?",
            )
            .bind(scheduler_draft.id)
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("scheduled publication state");
            if status == "published" {
                scheduler_published = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(scheduler_published);
        let scheduler_metrics = scheduler
            .shutdown()
            .await
            .expect("supervised publication scheduler shutdown");
        assert_eq!(scheduler_metrics.activated, 1);
        assert!(scheduler_metrics.cycles >= 1);
        assert!(scheduler_metrics.leadership_acquired >= 1);

        let replay = crate::services::publication_scheduler_service::run_cycle_at(
            &scheduler_actor,
            &scheduler_config,
            "publication-replay-token",
            scheduler_now + 2,
        )
        .await
        .expect("idempotent publication scheduler replay");
        assert_eq!(
            replay,
            crate::services::publication_scheduler_service::PublicationSchedulerOutcome::Completed {
                activated: 0,
            }
        );
        let (reviewed_by, publication_event_count, publication_payload) =
            rullst::db::sqlx::query_as::<_, (i32, i64, String)>(
                "SELECT reviewed_by, (SELECT COUNT(*) FROM academy_outbox WHERE event_key = ?), (SELECT payload_json FROM academy_outbox WHERE event_key = ?) FROM course_versions WHERE id = ?",
            )
            .bind("course-published:course-1-v4-scheduler")
            .bind("course-published:course-1-v4-scheduler")
            .bind(scheduler_draft.id)
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("scheduled publication audit evidence");
        assert_eq!(reviewed_by, 12);
        assert_eq!(publication_event_count, 1);
        let publication_payload: serde_json::Value =
            serde_json::from_str(&publication_payload).expect("publication event payload");
        assert_eq!(publication_payload["actor_user_id"], 21);

        rullst::db::sqlx::query(
            "UPDATE lesson_release_rules SET status = ?, release_at_epoch = 0, expire_at_epoch = 0 WHERE lesson_id = ?",
        )
        .bind("inactive")
        .bind(2_i32)
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("disable conflicting completion policies");
        rullst::db::sqlx::query(
            "UPDATE lesson_release_rules SET status = ? WHERE lesson_id = ? AND ruleset_version = ?",
        )
        .bind("active")
        .bind(2_i32)
        .bind("lesson-2-v1")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("restore canonical completion policy");
        rullst::db::sqlx::query(
            "INSERT INTO users (id, name, email, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(22_i32)
        .bind("Certificate Learner")
        .bind("certificate@example.test")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("certificate learner fixture");
        let certificate_learner = academy_context("22", vec!["student".to_string()]);
        enroll(22, &certificate_learner, 1)
            .await
            .expect("certificate learner enrollment");
        record_progress(
            &certificate_learner,
            22,
            1,
            100,
            "certificate-progress-lesson-1",
        )
        .await
        .expect("first certificate requirement");
        assert!(matches!(
            crate::services::completion_service::derive_completion_at(
                &certificate_learner,
                22,
                1,
                70_000,
            )
            .await,
            Err(crate::services::completion_service::CompletionError::Incomplete)
        ));
        assert!(matches!(
            crate::services::completion_service::derive_completion_at(
                &academy_context("8", vec!["student".to_string()]),
                22,
                1,
                70_000,
            )
            .await,
            Err(crate::services::completion_service::CompletionError::Forbidden)
        ));
        record_progress(
            &certificate_learner,
            22,
            2,
            100,
            "certificate-progress-lesson-2",
        )
        .await
        .expect("second certificate requirement");
        let completion = crate::services::completion_service::derive_completion_at(
            &certificate_learner,
            22,
            1,
            70_001,
        )
        .await
        .expect("derive pinned course completion");
        assert!(completion.applied);
        assert_eq!(completion.version_key, "course-1-v4-scheduler");
        assert_eq!(completion.ruleset_version, "course-1-completion-v4");
        let completion_replay = crate::services::completion_service::derive_completion_at(
            &certificate_learner,
            22,
            1,
            70_002,
        )
        .await
        .expect("completion replay");
        assert!(!completion_replay.applied);
        assert_eq!(completion_replay.certificate_key, completion.certificate_key);
        let verification = crate::services::completion_service::verify_certificate(
            &completion.certificate_key,
        )
        .await
        .expect("public certificate verification");
        assert!(verification.valid);
        assert_eq!(verification.course_id, 1);
        let public_json = serde_json::to_value(&verification).expect("public verification JSON");
        assert!(public_json.get("subject_user_id").is_none());
        let (evidence_json, completion_events) =
            rullst::db::sqlx::query_as::<_, (String, i64)>(
                "SELECT evidence_json, (SELECT COUNT(*) FROM academy_outbox WHERE event_kind = ? AND subject_user_id = ?) FROM course_completions WHERE id = ?",
            )
            .bind("course_completed")
            .bind(22_i32)
            .bind(completion.completion_id)
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("completion evidence and event");
        let evidence: serde_json::Value =
            serde_json::from_str(&evidence_json).expect("completion evidence JSON");
        assert_eq!(evidence["required_lesson_ids"], serde_json::json!([1, 2]));
        assert_eq!(completion_events, 1);

        let mismatched_completion = crate::controllers::completion_controller::complete(
            rullst::server::Path(1),
            rullst::server::Extension(23),
            rullst::server::Extension(certificate_learner.clone()),
        )
        .await;
        assert_eq!(mismatched_completion.status(), rullst::server::StatusCode::FORBIDDEN);
        let replay_response = crate::controllers::completion_controller::complete(
            rullst::server::Path(1),
            rullst::server::Extension(22),
            rullst::server::Extension(certificate_learner),
        )
        .await;
        assert_eq!(replay_response.status(), rullst::server::StatusCode::OK);

        let certificate_admin = academy_context("21", vec!["admin".to_string()]);
        assert!(matches!(
            crate::services::completion_service::revoke_certificate_at(
                &academy_context("22", vec!["student".to_string()]),
                "certificate-revoke-denied",
                &completion.certificate_key,
                70_010,
                "learner cannot revoke certificate",
            )
            .await,
            Err(crate::services::completion_service::CompletionError::Forbidden)
        ));
        let revoked = crate::services::completion_service::revoke_certificate_at(
            &certificate_admin,
            "certificate-revoke-22",
            &completion.certificate_key,
            70_011,
            "verified administrative correction",
        )
        .await
        .expect("audited certificate revocation");
        assert!(revoked.applied);
        assert!(!crate::services::completion_service::revoke_certificate_at(
            &certificate_admin,
            "certificate-revoke-22",
            &completion.certificate_key,
            70_012,
            "verified administrative correction",
        )
        .await
        .expect("certificate revocation replay")
        .applied);
        assert!(matches!(
            crate::services::completion_service::revoke_certificate_at(
                &certificate_admin,
                "certificate-revoke-conflict",
                &completion.certificate_key,
                70_013,
                "verified administrative correction",
            )
            .await,
            Err(crate::services::completion_service::CompletionError::IdempotencyConflict)
        ));
        let revoked_verification = crate::services::completion_service::verify_certificate(
            &completion.certificate_key,
        )
        .await
        .expect("revoked certificate remains verifiable");
        assert!(!revoked_verification.valid);
        assert_eq!(revoked_verification.revoked_at_epoch, 70_011);
        let verify_response = crate::controllers::completion_controller::verify(
            rullst::server::Path(completion.certificate_key.clone()),
        )
        .await;
        assert_eq!(verify_response.status(), rullst::server::StatusCode::GONE);
        let revoke_response = crate::controllers::completion_controller::revoke(
            rullst::server::Path(completion.certificate_key.clone()),
            rullst::server::Extension(certificate_admin),
            rullst::server::Json(
                crate::controllers::completion_controller::RevokeCertificatePayload {
                    revocation_key: "certificate-revoke-22".to_string(),
                    reason: "verified administrative correction".to_string(),
                },
            ),
        )
        .await;
        assert_eq!(revoke_response.status(), rullst::server::StatusCode::OK);
        let revocation_events = rullst::db::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM academy_outbox WHERE event_kind = ? AND subject_user_id = ?",
        )
        .bind("certificate_revoked")
        .bind(22_i32)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("certificate revocation event count");
        assert_eq!(revocation_events, 1);
"##;
