use super::*;

#[tokio::test]
async fn signed_mixed_outcomes_award_only_exact_answers_and_preserve_minimized_feedback() {
    let signer = signer(19);
    let f = Fixture::with_profile(1, profile(&signer)).await;
    let exercise = Exercise::new(
        scope(),
        id("mixed"),
        id("v1"),
        vec![
            GraderCase {
                id: id("hidden-correct"),
                input: [1, 2],
                expected: 3,
            },
            GraderCase {
                id: id("hidden-wrong"),
                input: [6, 8],
                expected: 14,
            },
            GraderCase {
                id: id("hidden-trap"),
                input: [2, 3],
                expected: 5,
            },
        ],
        ExecutionLimits::new(10, 100_000, 64).unwrap(),
    )
    .unwrap();
    f.store
        .register_exercise(&f.policy, &id("teacher"), &exercise)
        .await
        .unwrap();
    f.store
        .submit(
            &f.policy,
            &id("alice"),
            &scope(),
            Submission::new(
                id("mixed-job"),
                ExerciseRef::new("mixed", "v1").unwrap(),
                RustSource::new(SOURCE).unwrap(),
                300,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let job = f.store.claim_next().await.unwrap().unwrap();
    let signed = signer
        .sign(ExecutionReceipt {
            output: WorkerOutput {
                binding: job.input().binding().clone(),
                outcome: WorkerOutcome::Executed {
                    artifact: ContentHash::of(b"synthetic-test-artifact-not-execution"),
                    cases: vec![
                        CaseOutput::Value(3),
                        CaseOutput::Value(13),
                        CaseOutput::Trap(TrapKind::Fuel),
                    ],
                },
            },
            started_at: NOW,
            finished_at: NOW,
            observation_digest: ContentHash::of(b"synthetic-test-observation-not-isolation-proof"),
            teardown: Teardown::Confirmed,
        })
        .unwrap();
    let view = f
        .store
        .complete(job.scope(), job.id(), &signed)
        .await
        .unwrap();
    assert_eq!(view.state, JobState::Completed);
    let Some(JobResult::Graded {
        cases,
        passed,
        total,
        ..
    }) = &view.result
    else {
        panic!("expected the trusted grader's bounded result");
    };
    assert_eq!((*passed, *total), (1, 3));
    assert_eq!(
        cases,
        &[
            CaseGrade::Passed,
            CaseGrade::WrongAnswer,
            CaseGrade::Trapped(TrapKind::Fuel)
        ]
    );
    let persisted = f
        .store
        .get_job(&f.policy, &id("alice"), job.scope(), job.id())
        .await
        .unwrap();
    assert_eq!(persisted, view);
    let public = serde_json::to_string(&persisted).unwrap();
    for hidden in [
        "expected",
        "input",
        "hidden-correct",
        "hidden-wrong",
        "hidden-trap",
        SOURCE,
        "Value",
    ] {
        assert!(!public.contains(hidden));
    }
    assert_eq!(
        f.store
            .complete(job.scope(), job.id(), &signed)
            .await
            .unwrap(),
        view
    );
    f.store.close().await;
}
