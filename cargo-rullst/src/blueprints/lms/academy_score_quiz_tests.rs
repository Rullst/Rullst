// Materialized quiz/score continuation appended to the generated Academy test.

pub const GENERATED_SCORE_QUIZ_TESTS_SUFFIX: &str = r##"
        let quiz_submission = QuizSubmission {
            attempt_key: "quiz-attempt-sqlite-1".to_string(),
            quiz_id: 1,
            subject_user_id: 7,
            ruleset_version: "memory-rules-v1".to_string(),
            answers: vec![QuizAnswerInput {
                question_id: 1,
                option_id: 1,
            }],
        };
        let quiz_grade = grade_quiz_at(&learner, quiz_submission.clone(), 3_000)
            .await
            .expect("authoritative quiz grade");
        assert!(quiz_grade.applied);
        assert!(quiz_grade.passed);
        assert_eq!(quiz_grade.score_percent, 100);
        let quiz_replay = grade_quiz_at(&learner, quiz_submission.clone(), 3_001)
            .await
            .expect("quiz replay");
        assert!(!quiz_replay.applied);
        assert!(matches!(
            grade_quiz_at(
                &academy_context("8", vec!["student".to_string()]),
                quiz_submission,
                3_002,
            )
            .await,
            Err(AssessmentError::Access(_))
        ));

        let tampered = QuizSubmission {
            attempt_key: "quiz-attempt-tampered".to_string(),
            quiz_id: 1,
            subject_user_id: 7,
            ruleset_version: "memory-rules-v1".to_string(),
            answers: vec![QuizAnswerInput {
                question_id: 1,
                option_id: 999,
            }],
        };
        assert!(matches!(
            grade_quiz_at(&learner, tampered, 3_003).await,
            Err(AssessmentError::InvalidField("unknown option"))
        ));
        for number in 2..=3 {
            let incorrect = QuizSubmission {
                attempt_key: format!("quiz-attempt-sqlite-{number}"),
                quiz_id: 1,
                subject_user_id: 7,
                ruleset_version: "memory-rules-v1".to_string(),
                answers: vec![QuizAnswerInput {
                    question_id: 1,
                    option_id: 2,
                }],
            };
            let grade = grade_quiz_at(&learner, incorrect, 3_000 + i64::from(number))
                .await
                .expect("bounded quiz attempt");
            assert_eq!(grade.score_percent, 0);
        }
        let over_limit = QuizSubmission {
            attempt_key: "quiz-attempt-sqlite-4".to_string(),
            quiz_id: 1,
            subject_user_id: 7,
            ruleset_version: "memory-rules-v1".to_string(),
            answers: vec![QuizAnswerInput {
                question_id: 1,
                option_id: 1,
            }],
        };
        assert!(matches!(
            grade_quiz_at(&learner, over_limit, 3_004).await,
            Err(AssessmentError::AttemptLimit)
        ));
        let (attempt_count, answer_count, quiz_event_count, quiz_score_count, score_event_count) =
            rullst::db::sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
                "SELECT (SELECT COUNT(*) FROM quiz_attempts), (SELECT COUNT(*) FROM quiz_answers), (SELECT COUNT(*) FROM academy_outbox WHERE event_kind = ?), (SELECT COUNT(*) FROM score_events WHERE origin = ?), (SELECT COUNT(*) FROM academy_outbox WHERE event_kind = ?)",
            )
            .bind("quiz_graded")
            .bind("quiz")
            .bind("score_recorded")
            .fetch_one(Orm::pool().expect("Academy pool"))
            .await
            .expect("quiz persistence counts");
        assert_eq!(
            (
                attempt_count,
                answer_count,
                quiz_event_count,
                quiz_score_count,
                score_event_count,
            ),
            (3, 3, 3, 3, 4)
        );
        assert!(!crate::services::score_service::leaderboard_cache_contains(
            &learner,
            1,
            "season-2026",
        )
        .await
        .expect("authoritative quiz invalidates leaderboard cache"));
        let quiz_ranked = leaderboard(&learner, 1, "season-2026", 10)
            .await
            .expect("quiz-updated leaderboard");
        assert_eq!(quiz_ranked[0].score, 190);
"##;
