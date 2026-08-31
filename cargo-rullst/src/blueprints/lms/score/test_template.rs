//! Tests emitted inside the generated score-service module.

pub(super) const SCORE_TESTS: &str = r##"use super::*;

fn submission() -> ScoreSubmission {
    ScoreSubmission {
        idempotency_key: "event-1".to_string(),
        schema_version: SCORE_EVENT_SCHEMA_VERSION,
        origin: "activity".to_string(),
        subject_user_id: 7,
        course_id: 2,
        activity_id: 3,
        attempt_key: "attempt-1".to_string(),
        points: 80,
        max_score: 100,
        ruleset_version: "rules-v1".to_string(),
        season_key: "season-2026".to_string(),
        evidence_sha256: "a".repeat(64),
    }
}

#[test]
fn actor_comes_from_authenticated_context_and_cross_user_is_denied() {
    let owner = UserContext::new("7", vec!["student".to_string()]);
    let attacker = UserContext::new("8", vec!["student".to_string()]);

    let validated = validate(&owner, submission()).expect("owner score should validate");
    assert_eq!(validated.actor_user_id, 7);
    assert!(matches!(
        validate(&attacker, submission()),
        Err(ScoreError::Forbidden)
    ));
}

#[test]
fn invalid_schema_keys_scores_and_evidence_fail_closed() {
    let owner = UserContext::new("7", vec!["student".to_string()]);
    let mut invalid = submission();
    invalid.points = 101;
    assert!(matches!(
        validate(&owner, invalid),
        Err(ScoreError::InvalidField("score bounds"))
    ));

    let mut invalid_evidence = submission();
    invalid_evidence.evidence_sha256 = "A".repeat(64);
    assert!(matches!(
        validate(&owner, invalid_evidence),
        Err(ScoreError::InvalidField("evidence_sha256"))
    ));

    let mut future = submission();
    future.schema_version = 3;
    assert!(matches!(
        validate(&owner, future),
        Err(ScoreError::UnsupportedSchemaVersion(3))
    ));
}
"##;
