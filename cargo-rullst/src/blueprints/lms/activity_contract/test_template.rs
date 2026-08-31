//! Tests emitted inside the generated activity-contract module.

pub(super) const ACTIVITY_CONTRACT_TESTS: &str = r##"use super::*;

fn attempt() -> ActivityAttempt {
    ActivityAttempt {
        schema_version: ACTIVITY_SCHEMA_VERSION,
        attempt_key: "attempt-1".to_string(),
        activity_id: 2,
        subject_user_id: 7,
        kind: ActivityKind::Exercise,
        ruleset_version: "rules-v1".to_string(),
        started_at_epoch_seconds: 1_000,
        state_json: "{\"level\":1}".to_string(),
    }
}

fn evaluator() -> SingleChoiceEvaluator {
    SingleChoiceEvaluator::new(
        11,
        100,
        "a".repeat(64),
        r#"{"schema_version":1,"mode":"single_choice","correct_option_id":11}"#,
    )
        .expect("trusted single-choice rules")
}

#[test]
fn single_choice_result_is_server_authored_and_bound_to_the_owner() {
    let owner = UserContext::new("7", vec!["student".to_string()]);
    let attacker = UserContext::new("8", vec!["student".to_string()]);
    let correct = evaluate_activity(
        &owner,
        attempt(),
        &SingleChoiceSubmission {
            selected_option_id: 11,
        },
        1_100,
        &evaluator(),
    )
    .expect("server-authored correct result");
    assert_eq!(correct.result().points, 100);
    assert_eq!(correct.result().submission_key, "option:11");

    let incorrect = evaluate_activity(
        &owner,
        attempt(),
        &SingleChoiceSubmission {
            selected_option_id: 12,
        },
        1_100,
        &evaluator(),
    )
    .expect("server-authored incorrect result");
    assert_eq!(incorrect.result().points, 0);
    assert!(matches!(
        evaluate_activity(
            &attacker,
            attempt(),
            &SingleChoiceSubmission {
                selected_option_id: 11,
            },
            1_100,
            &evaluator(),
        ),
        Err(ActivityContractError::Forbidden)
    ));

    let mut mismatched_kind = attempt();
    mismatched_kind.kind = ActivityKind::Game;
    assert!(matches!(
        evaluate_activity(
            &owner,
            mismatched_kind,
            &SingleChoiceSubmission {
                selected_option_id: 11,
            },
            1_100,
            &evaluator(),
        ),
        Err(ActivityContractError::InvalidField("activity kind"))
    ));
    assert!(SingleChoiceEvaluator::new(11, 100, "A".repeat(64), "{}").is_err());
}

#[test]
fn matching_result_is_order_independent_bounded_and_server_scored() {
    let owner = UserContext::new("7", vec!["student".to_string()]);
    let policy = r#"{"schema_version":1,"mode":"matching","pairs":[{"left_id":1,"right_id":11},{"left_id":2,"right_id":12},{"left_id":3,"right_id":13}]}"#;
    let evaluator = MatchingEvaluator::new(
        vec![
            MatchingPair { left_id: 1, right_id: 11 },
            MatchingPair { left_id: 2, right_id: 12 },
            MatchingPair { left_id: 3, right_id: 13 },
        ],
        90,
        "e".repeat(64),
        policy,
    )
    .expect("trusted matching policy");
    let partially_correct = vec![
        MatchingPair { left_id: 3, right_id: 13 },
        MatchingPair { left_id: 1, right_id: 12 },
        MatchingPair { left_id: 2, right_id: 11 },
    ];
    let result = evaluate_activity(
        &owner,
        attempt(),
        partially_correct.as_slice(),
        1_100,
        &evaluator,
    )
    .expect("bounded matching result");
    assert_eq!(result.result().points, 30);
    assert_eq!(result.result().submission_key, "pairs:1-12.2-11.3-13");
    let duplicate_left = vec![
        MatchingPair { left_id: 1, right_id: 11 },
        MatchingPair { left_id: 1, right_id: 12 },
        MatchingPair { left_id: 3, right_id: 13 },
    ];
    assert!(matches!(
        evaluate_activity(
            &owner,
            attempt(),
            duplicate_left.as_slice(),
            1_100,
            &evaluator,
        ),
        Err(ActivityContractError::InvalidField("matching submission"))
    ));
}

#[test]
fn typed_result_normalizes_hashes_and_scores_without_retaining_raw_input() {
    let owner = UserContext::new("7", vec!["student".to_string()]);
    let policy = r#"{"schema_version":1,"mode":"typed_answer","case_sensitive":false,"accepted_answers":["ownership","borrow checker"]}"#;
    let evaluator = TypedAnswerEvaluator::new(
        vec!["ownership".to_string(), "borrow checker".to_string()],
        false,
        70,
        "f".repeat(64),
        policy,
    )
    .expect("trusted typed-answer policy");
    let correct = evaluate_activity(
        &owner,
        attempt(),
        "  OWNERSHIP  ",
        1_100,
        &evaluator,
    )
    .expect("normalized correct typed answer");
    let normalized_replay = evaluate_activity(
        &owner,
        attempt(),
        "ownership",
        1_100,
        &evaluator,
    )
    .expect("canonical typed answer");
    assert_eq!(correct.result().points, 70);
    assert_eq!(
        correct.result().submission_key,
        normalized_replay.result().submission_key
    );
    assert!(correct.result().submission_key.starts_with("text:"));
    assert!(!correct.result().submission_key.contains("ownership"));
    let wrong = evaluate_activity(&owner, attempt(), "borrowing", 1_100, &evaluator)
        .expect("bounded incorrect typed answer");
    assert_eq!(wrong.result().points, 0);
    assert_ne!(wrong.result().submission_key, correct.result().submission_key);
    assert!(matches!(
        evaluate_activity(&owner, attempt(), "line\nbreak", 1_100, &evaluator),
        Err(ActivityContractError::InvalidField("typed answer"))
    ));
}

#[test]
fn client_boundary_defaults_to_zero_bundle_and_bounds_opt_in_wasm() {
    let simple = ActivityClientManifest::ssr_htmx("/activities/2/play")
        .expect("same-origin SSR activity");
    assert_eq!(simple.kind, ActivityClientKind::SsrHtmx);
    assert!(simple.wasm_path.is_none());

    let rich = ActivityClientManifest::canvas_wasm(
        "/activities/3/play",
        "/assets/games/borrow-checker.wasm",
        "a".repeat(64),
        512_000,
    )
    .expect("bounded same-origin Wasm activity");
    assert_eq!(rich.kind, ActivityClientKind::CanvasWasm);

    assert!(matches!(
        ActivityClientManifest::canvas_wasm(
            "/activities/3/play",
            "https://attacker.example/game.wasm",
            "a".repeat(64),
            512_000,
        ),
        Err(ActivityContractError::InvalidField("wasm_path"))
    ));
    assert!(matches!(
        ActivityClientManifest::canvas_wasm(
            "/activities/3/play",
            "/assets/game.wasm",
            "A".repeat(64),
            17 * 1024 * 1024,
        ),
        Err(ActivityContractError::InvalidField("wasm_sha256"))
    ));
}
"##;
