// Assignment/rubric continuation of the generated materialized Academy test.

pub const GENERATED_ASSIGNMENT_TESTS_SUFFIX: &str = r##"
        let assignment_learner = academy_context("22", vec!["student".to_string()]);
        rullst::db::sqlx::query("UPDATE assignments SET due_at_epoch = ? WHERE id = ?")
            .bind(70_029_i64)
            .bind(1_i32)
            .execute(Orm::pool().expect("Academy pool"))
            .await
            .expect("set assignment deadline");
        assert!(matches!(
            crate::services::assignment_submission_service::submit_assignment_at(
                &assignment_learner,
                crate::services::assignment_submission_service::AssignmentSubmissionInput {
                    submission_key: "assignment-late-22".to_string(), assignment_id: 1,
                    subject_user_id: 22, content_text: "late response".to_string(),
                },
                70_030,
            ).await,
            Err(crate::services::assignment_submission_service::AssignmentSubmissionError::Deadline)
        ));
        rullst::db::sqlx::query("UPDATE assignments SET due_at_epoch = 0 WHERE id = ?")
            .bind(1_i32)
            .execute(Orm::pool().expect("Academy pool"))
            .await
            .expect("restore assignment deadline");
        assert!(matches!(
            crate::services::assignment_submission_service::submit_assignment_at(
                &academy_context("8", vec!["student".to_string()]),
                crate::services::assignment_submission_service::AssignmentSubmissionInput {
                    submission_key: "assignment-cross-user".to_string(), assignment_id: 1,
                    subject_user_id: 22, content_text: "forged response".to_string(),
                },
                70_031,
            ).await,
            Err(crate::services::assignment_submission_service::AssignmentSubmissionError::Forbidden)
        ));
        let assignment_input =
            crate::services::assignment_submission_service::AssignmentSubmissionInput {
                submission_key: "assignment-submission-22".to_string(), assignment_id: 1,
                subject_user_id: 22,
                content_text: "Ownership was transferred after the borrow; keep the owner alive and borrow within its scope.".to_string(),
            };
        let assignment_submission =
            crate::services::assignment_submission_service::submit_assignment_at(
                &assignment_learner, assignment_input.clone(), 70_031,
            ).await.expect("owner-bound assignment submission");
        assert!(assignment_submission.applied);
        assert!(!crate::services::assignment_submission_service::submit_assignment_at(
            &assignment_learner, assignment_input.clone(), 70_032,
        ).await.expect("assignment submission replay").applied);
        let mut conflicting_submission = assignment_input.clone();
        conflicting_submission.content_text = "different response".to_string();
        assert!(matches!(
            crate::services::assignment_submission_service::submit_assignment_at(
                &assignment_learner, conflicting_submission, 70_033,
            ).await,
            Err(crate::services::assignment_submission_service::AssignmentSubmissionError::IdempotencyConflict)
        ));

        rullst::db::sqlx::query(
            "INSERT INTO users (id, name, email, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(30_i32)
        .bind("Academy Evaluator")
        .bind("evaluator@example.test")
        .execute(Orm::pool().expect("Academy pool"))
        .await
        .expect("assignment evaluator fixture");
        let assignment_grade_input =
            crate::services::assignment_grading_service::AssignmentGradeInput {
                grading_key: "assignment-grade-22".to_string(),
                submission_id: assignment_submission.submission_id,
                feedback: "Clear diagnosis and a safe remediation.".to_string(),
                scores: vec![
                    crate::services::assignment_grading_service::RubricScoreInput {
                        criterion_id: 1, points_awarded: 50,
                        feedback: "Identifies the ownership lifetime issue.".to_string(),
                    },
                    crate::services::assignment_grading_service::RubricScoreInput {
                        criterion_id: 2, points_awarded: 35,
                        feedback: "The remediation is safe and actionable.".to_string(),
                    },
                ],
            };
        assert!(matches!(
            crate::services::assignment_grading_service::grade_assignment_at(
                &assignment_learner, assignment_grade_input.clone(), 70_034,
            ).await,
            Err(crate::services::assignment_grading_service::AssignmentGradeError::Forbidden)
        ));
        let evaluator = academy_context("30", vec!["evaluator".to_string()]);
        let mut impossible_grade = assignment_grade_input.clone();
        impossible_grade.grading_key = "assignment-grade-impossible".to_string();
        impossible_grade.scores[0].points_awarded = 61;
        assert!(matches!(
            crate::services::assignment_grading_service::grade_assignment_at(
                &evaluator, impossible_grade, 70_034,
            ).await,
            Err(crate::services::assignment_grading_service::AssignmentGradeError::InvalidScore)
        ));
        let assignment_grade = crate::services::assignment_grading_service::grade_assignment_at(
            &evaluator, assignment_grade_input.clone(), 70_035,
        ).await.expect("server-bounded rubric grade");
        assert!(assignment_grade.applied);
        assert_eq!((assignment_grade.points_awarded, assignment_grade.max_points), (85, 100));
        assert!(!crate::services::assignment_grading_service::grade_assignment_at(
            &evaluator, assignment_grade_input.clone(), 70_036,
        ).await.expect("assignment grade replay").applied);
        let mut conflicting_grade = assignment_grade_input.clone();
        conflicting_grade.scores[1].points_awarded = 34;
        assert!(matches!(
            crate::services::assignment_grading_service::grade_assignment_at(
                &evaluator, conflicting_grade, 70_037,
            ).await,
            Err(crate::services::assignment_grading_service::AssignmentGradeError::IdempotencyConflict)
        ));

        let submission_http = crate::controllers::assignment_controller::submit(
            rullst::server::Path(1), rullst::server::Extension(22),
            rullst::server::Extension(assignment_learner),
            rullst::server::Json(crate::controllers::assignment_controller::AssignmentSubmissionPayload {
                submission_key: assignment_input.submission_key,
                content_text: assignment_input.content_text,
            }),
        ).await;
        assert_eq!(submission_http.status(), rullst::server::StatusCode::OK);
        let grade_http = crate::controllers::assignment_controller::grade(
            rullst::server::Path(assignment_submission.submission_id),
            rullst::server::Extension(evaluator),
            rullst::server::Json(crate::controllers::assignment_controller::AssignmentGradePayload {
                grading_key: assignment_grade_input.grading_key,
                feedback: assignment_grade_input.feedback,
                scores: assignment_grade_input.scores.into_iter().map(|score|
                    crate::controllers::assignment_controller::RubricScorePayload {
                        criterion_id: score.criterion_id,
                        points_awarded: score.points_awarded,
                        feedback: score.feedback,
                    }
                ).collect(),
            }),
        ).await;
        assert_eq!(grade_http.status(), rullst::server::StatusCode::OK);

        let correction_input =
            crate::services::assignment_grade_correction_service::AssignmentGradeCorrectionInput {
                correction_key: "assignment-correction-22".to_string(),
                assignment_grade_id: assignment_grade.grade_id,
                reason: "administrative moderation after rubric review".to_string(),
                scores: vec![
                    crate::services::assignment_grading_service::RubricScoreInput {
                        criterion_id: 1, points_awarded: 45,
                        feedback: "Moderated analysis score.".to_string(),
                    },
                    crate::services::assignment_grading_service::RubricScoreInput {
                        criterion_id: 2, points_awarded: 35,
                        feedback: "Remediation score remains unchanged.".to_string(),
                    },
                ],
            };
        assert!(matches!(
            crate::services::assignment_grade_correction_service::correct_assignment_grade_at(
                &academy_context("30", vec!["evaluator".to_string()]),
                correction_input.clone(), 70_038,
            ).await,
            Err(crate::services::assignment_grade_correction_service::AssignmentGradeCorrectionError::Forbidden)
        ));
        let correction_admin = academy_context("21", vec!["admin".to_string()]);
        let mut impossible_correction = correction_input.clone();
        impossible_correction.correction_key = "assignment-correction-impossible".to_string();
        impossible_correction.scores[0].points_awarded = 61;
        assert!(matches!(
            crate::services::assignment_grade_correction_service::correct_assignment_grade_at(
                &correction_admin, impossible_correction, 70_038,
            ).await,
            Err(crate::services::assignment_grade_correction_service::AssignmentGradeCorrectionError::InvalidScore)
        ));
        let correction =
            crate::services::assignment_grade_correction_service::correct_assignment_grade_at(
                &correction_admin, correction_input.clone(), 70_039,
            ).await.expect("append-only assignment grade correction");
        assert!(correction.applied);
        assert_eq!((correction.previous_points, correction.corrected_points), (85, 80));
        assert_eq!(
            crate::services::assignment_grade_correction_service::effective_grade(
                assignment_grade.grade_id,
            ).await.expect("effective corrected assignment grade"),
            (80, 100),
        );
        assert!(!crate::services::assignment_grade_correction_service::correct_assignment_grade_at(
            &correction_admin, correction_input.clone(), 70_040,
        ).await.expect("assignment grade correction replay").applied);
        let mut conflicting_correction = correction_input.clone();
        conflicting_correction.scores[0].points_awarded = 44;
        assert!(matches!(
            crate::services::assignment_grade_correction_service::correct_assignment_grade_at(
                &correction_admin, conflicting_correction, 70_041,
            ).await,
            Err(crate::services::assignment_grade_correction_service::AssignmentGradeCorrectionError::IdempotencyConflict)
        ));
        let correction_http = crate::controllers::assignment_controller::correct_grade(
            rullst::server::Path(assignment_grade.grade_id),
            rullst::server::Extension(correction_admin),
            rullst::server::Json(
                crate::controllers::assignment_controller::AssignmentGradeCorrectionPayload {
                    correction_key: correction_input.correction_key,
                    reason: correction_input.reason,
                    scores: correction_input.scores.into_iter().map(|score|
                        crate::controllers::assignment_controller::RubricScorePayload {
                            criterion_id: score.criterion_id,
                            points_awarded: score.points_awarded,
                            feedback: score.feedback,
                        }
                    ).collect(),
                },
            ),
        ).await;
        assert_eq!(correction_http.status(), rullst::server::StatusCode::OK);
        let correction_audit = rullst::db::sqlx::query_as::<_, (i32, i32, i32, String, i64)>(
            "SELECT actor_user_id, previous_points, corrected_points, reason, corrected_at_epoch FROM assignment_grade_corrections WHERE correction_key = ?",
        )
        .bind("assignment-correction-22")
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("assignment grade correction audit");
        assert_eq!(correction_audit, (
            21, 85, 80, "administrative moderation after rubric review".to_string(), 70_039,
        ));

        let assignment_evidence = rullst::db::sqlx::query_as::<_, (String, i64, i64, i64, i64)>(
            "SELECT status, (SELECT COUNT(*) FROM assignment_grades WHERE submission_id = ?), (SELECT COUNT(*) FROM rubric_scores WHERE assignment_grade_id = ?), (SELECT COUNT(*) FROM assignment_grade_corrections WHERE assignment_grade_id = ?), (SELECT COUNT(*) FROM academy_outbox WHERE event_kind IN (?, ?, ?) AND subject_user_id = ?)
             FROM assignment_submissions WHERE id = ?",
        )
        .bind(assignment_submission.submission_id)
        .bind(assignment_grade.grade_id)
        .bind(assignment_grade.grade_id)
        .bind("assignment_submitted")
        .bind("assignment_graded")
        .bind("assignment_grade_corrected")
        .bind(22_i32)
        .bind(assignment_submission.submission_id)
        .fetch_one(Orm::pool().expect("Academy pool"))
        .await
        .expect("assignment rubric evidence");
        assert_eq!(assignment_evidence, ("graded".to_string(), 1, 2, 1, 3));
"##;
