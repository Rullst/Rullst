# 46. Server-Authoritative Learning Activities

Interactive learning clients must never decide their own points. The complete
Academy starter generates a static-dispatch `ActivityEvaluator` boundary that
turns an untrusted submission into a server-authored `ActivityResult`.

## Evaluate a single-choice exercise

Load the correct option, maximum score and canonical SHA-256 digest from your
trusted, versioned rules. Do not accept them from an HTTP form.

```rust,no_run
use crate::services::activity_contract::{
    ActivityAttempt, ActivityKind, SingleChoiceEvaluator,
    SingleChoiceSubmission, ACTIVITY_SCHEMA_VERSION, evaluate_activity,
};
use crate::services::score_service::{ScoreReceipt, record_activity_result};
use rullst_security::UserContext;

async fn grade(
    context: &UserContext,
    selected_option_id: i32,
) -> Result<ScoreReceipt, Box<dyn std::error::Error>> {
    // These three values represent data loaded from trusted server state.
    let evaluator = SingleChoiceEvaluator::new(7, 100, "a".repeat(64))?;
    let attempt = ActivityAttempt {
        schema_version: ACTIVITY_SCHEMA_VERSION,
        attempt_key: "attempt-language-1".to_string(),
        activity_id: 42,
        subject_user_id: 9,
        kind: ActivityKind::Exercise,
        ruleset_version: "portuguese-a1-v3".to_string(),
        started_at_epoch_seconds: 1_800_000_000,
        state_json: r#"{"prompt_version":3}"#.to_string(),
    };
    let validated = evaluate_activity(
        context,
        attempt,
        &SingleChoiceSubmission { selected_option_id },
        1_800_000_030,
        &evaluator,
    )?;
    Ok(record_activity_result(context, validated).await?)
}
```

The submission contains an option ID but no score, maximum or answer key. The
contract rejects cross-owner access, activity-kind mismatch, invalid identity,
non-object or oversized state, reversed time, out-of-range results and
non-canonical evidence digests.

## Persist one authoritative transaction

In the complete Academy starter, pass the opaque `ValidatedActivityResult`
directly to `record_activity_result`. Callers cannot construct that wrapper or
replace its points. The bridge loads the persisted activity and rechecks its
course, kind, maximum, ruleset, season, evidence digest and exact evaluator
configuration before one transaction:

1. appends a deduplicated `ScoreEvent` v2;
2. updates the authoritative leaderboard projection; and
3. appends the strict `score_recorded` v2 outbox event before commit.

The configuration is checked again under the transaction's policy lock. A
concurrent answer-policy edit therefore cannot commit a result graded against
stale rules.

The complete starter also persists the bounded attempt/result in that
transaction. Identical retries are no-ops; changing the selected option under
the same attempt key is a conflict. The database scopes that client key by
learner and activity, while the score-event key is derived server-side, so keys
chosen by different learners cannot reserve one another's attempts.

## Submit through the authenticated route

The generated owner-only route is `POST /activities/{id}/attempts`. After your
normal authenticated session and CSRF ceremony, its JSON body contains only:

```json
{
  "attempt_key": "attempt-language-1",
  "selected_option_id": 7
}
```

Do not add learner ID, ruleset, answer key, points, maximum, evidence or client
time to this payload. The route and persisted activity supply them. The stored
bounded `state_json` is hidden in Nexus but is still application data: include
`activity_attempts` in retention, export and erasure policy where applicable.

The full Academy quiz service follows the same score-event invariants but is
still a separate evaluator. Do not claim generic quiz or game integration until
those paths are unified and their materialized tests pass.

Implement another `ActivityEvaluator` when a spelling, listening or matching
exercise needs different trusted rules. Keep the concrete evaluator type
visible; do not use a runtime registry merely to hide domain differences.
