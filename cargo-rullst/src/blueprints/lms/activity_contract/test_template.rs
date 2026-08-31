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
    SingleChoiceEvaluator::new(11, 100, "a".repeat(64))
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
    assert!(SingleChoiceEvaluator::new(11, 100, "A".repeat(64)).is_err());
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
