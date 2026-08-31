// Versioned, fail-closed automation planning for the Academy starter.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/services/automation_service.rs",
        AUTOMATION_SERVICE.to_string(),
    )]
}

const AUTOMATION_SERVICE: &str = r##"use serde::Deserialize;

pub const AUTOMATION_SCHEMA_VERSION: i32 = 1;
pub const SCORE_RECORDED_SCHEMA_VERSION: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomationRuleInput {
    pub id: i32,
    pub enabled: bool,
    pub trigger_kind: String,
    pub action_kind: String,
    pub config_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannedAction {
    AwardAchievement {
        subject_user_id: i32,
        achievement_code: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomationPlan {
    pub execution_key: String,
    pub rule_id: i32,
    pub source_event_key: String,
    pub actor_user_id: i32,
    pub action: PlannedAction,
}

#[derive(Debug)]
pub enum AutomationError {
    InvalidField(&'static str),
    UnsupportedSchemaVersion(i32),
    UnsupportedAction(String),
    InvalidJson(serde_json::Error),
}

impl std::fmt::Display for AutomationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField(field) => write!(formatter, "invalid automation field: {field}"),
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported automation schema version: {version}")
            }
            Self::UnsupportedAction(action) => {
                write!(formatter, "unsupported automation action: {action}")
            }
            Self::InvalidJson(error) => write!(formatter, "invalid automation JSON: {error}"),
        }
    }
}

impl std::error::Error for AutomationError {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScoreRecordedV2 {
    schema_version: i32,
    idempotency_key: String,
    origin: String,
    actor_user_id: i32,
    subject_user_id: i32,
    course_id: i32,
    activity_id: i32,
    attempt_key: String,
    points: i32,
    max_score: i32,
    ruleset_version: String,
    season_key: String,
    evidence_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AwardAchievementV1 {
    schema_version: i32,
    achievement_code: String,
    minimum_score: i32,
}

/// Produces a side-effect-free plan. Executors must insert `execution_key` into
/// a unique durable table in the same transaction as the selected action.
pub fn plan_score_automations(
    event_key: &str,
    event_kind: &str,
    payload_json: &str,
    rules: &[AutomationRuleInput],
) -> Result<Vec<AutomationPlan>, AutomationError> {
    if !valid_key(event_key, 160) || event_kind != "score_recorded" || payload_json.len() > 16_384 {
        return Err(AutomationError::InvalidField("event envelope"));
    }
    let event: ScoreRecordedV2 =
        serde_json::from_str(payload_json).map_err(AutomationError::InvalidJson)?;
    validate_event(&event)?;

    let mut plans = Vec::new();
    for rule in rules {
        if !rule.enabled || rule.trigger_kind != event_kind {
            continue;
        }
        if rule.id <= 0 || rule.config_json.len() > 8_192 {
            return Err(AutomationError::InvalidField("rule"));
        }
        if rule.action_kind != "award_achievement" {
            return Err(AutomationError::UnsupportedAction(rule.action_kind.clone()));
        }
        let config: AwardAchievementV1 =
            serde_json::from_str(&rule.config_json).map_err(AutomationError::InvalidJson)?;
        if config.schema_version != AUTOMATION_SCHEMA_VERSION {
            return Err(AutomationError::UnsupportedSchemaVersion(config.schema_version));
        }
        if !valid_key(&config.achievement_code, 64)
            || config.minimum_score < 0
            || config.minimum_score > 1_000_000
        {
            return Err(AutomationError::InvalidField("award_achievement config"));
        }
        if event.points >= config.minimum_score {
            plans.push(AutomationPlan {
                execution_key: format!("automation:{}:{}", rule.id, event_key),
                rule_id: rule.id,
                source_event_key: event_key.to_string(),
                actor_user_id: event.actor_user_id,
                action: PlannedAction::AwardAchievement {
                    subject_user_id: event.subject_user_id,
                    achievement_code: config.achievement_code,
                },
            });
        }
    }
    plans.sort_by_key(|plan| plan.rule_id);
    Ok(plans)
}

fn validate_event(event: &ScoreRecordedV2) -> Result<(), AutomationError> {
    if event.schema_version != SCORE_RECORDED_SCHEMA_VERSION {
        return Err(AutomationError::UnsupportedSchemaVersion(event.schema_version));
    }
    for (field, value, maximum) in [
        ("idempotency_key", event.idempotency_key.as_str(), 128),
        ("attempt_key", event.attempt_key.as_str(), 128),
        ("ruleset_version", event.ruleset_version.as_str(), 64),
        ("season_key", event.season_key.as_str(), 64),
    ] {
        if !valid_key(value, maximum) {
            return Err(AutomationError::InvalidField(field));
        }
    }
    if !matches!(event.origin.as_str(), "quiz" | "activity" | "game")
        || event.subject_user_id <= 0
        || event.actor_user_id <= 0
        || event.course_id <= 0
        || event.activity_id <= 0
        || event.points < 0
        || event.max_score <= 0
        || event.points > event.max_score
        || event.max_score > 1_000_000
        || !valid_sha256(&event.evidence_sha256)
    {
        return Err(AutomationError::InvalidField("score event"));
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event_with_max(points: i32, max_score: i32) -> String {
        serde_json::json!({
            "schema_version": 2,
            "idempotency_key": "score-1",
            "origin": "game",
            "actor_user_id": 7,
            "subject_user_id": 7,
            "course_id": 1,
            "activity_id": 1,
            "attempt_key": "attempt-1",
            "points": points,
            "max_score": max_score,
            "ruleset_version": "rules-v1",
            "season_key": "season-2026",
            "evidence_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        }).to_string()
    }

    fn event(points: i32) -> String {
        event_with_max(points, 100)
    }

    fn rule() -> AutomationRuleInput {
        AutomationRuleInput {
            id: 4,
            enabled: true,
            trigger_kind: "score_recorded".to_string(),
            action_kind: "award_achievement".to_string(),
            config_json: serde_json::json!({
                "schema_version": 1,
                "achievement_code": "memory-guardian",
                "minimum_score": 80,
            }).to_string(),
        }
    }

    #[test]
    fn planning_is_deterministic_idempotency_keyed_and_side_effect_free() {
        let plans = plan_score_automations(
            "score:event-1",
            "score_recorded",
            &event(80),
            &[rule()],
        ).expect("automation plan");
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].execution_key, "automation:4:score:event-1");
        assert_eq!(plans[0].source_event_key, "score:event-1");
        assert_eq!(plans[0].actor_user_id, 7);
        assert_eq!(
            plans[0].action,
            PlannedAction::AwardAchievement {
                subject_user_id: 7,
                achievement_code: "memory-guardian".to_string(),
            }
        );
        assert!(plan_score_automations(
            "score:event-1",
            "score_recorded",
            &event(79),
            &[rule()],
        ).expect("below-threshold plan").is_empty());
    }

    #[test]
    fn threshold_above_activity_max_is_a_valid_non_match() {
        let plans = plan_score_automations(
            "score:low-max",
            "score_recorded",
            &event_with_max(70, 70),
            &[rule()],
        )
        .expect("a globally valid threshold may exceed one activity maximum");
        assert!(plans.is_empty());
    }

    #[test]
    fn unknown_fields_versions_actions_and_impossible_scores_fail_closed() {
        let mut unsupported = rule();
        unsupported.action_kind = "send_money".to_string();
        assert!(matches!(
            plan_score_automations(
                "score:event-1",
                "score_recorded",
                &event(80),
                &[unsupported],
            ),
            Err(AutomationError::UnsupportedAction(_))
        ));

        let future = event(80).replace("\"schema_version\":2", "\"schema_version\":3");
        assert!(matches!(
            plan_score_automations("score:event-1", "score_recorded", &future, &[rule()]),
            Err(AutomationError::UnsupportedSchemaVersion(3))
        ));

        let impossible = event(101);
        assert!(matches!(
            plan_score_automations("score:event-1", "score_recorded", &impossible, &[rule()]),
            Err(AutomationError::InvalidField("score event"))
        ));

        let invalid_evidence = event(80).replace(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        );
        assert!(matches!(
            plan_score_automations(
                "score:event-1",
                "score_recorded",
                &invalid_evidence,
                &[rule()],
            ),
            Err(AutomationError::InvalidField("score event"))
        ));
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::AUTOMATION_SERVICE;

    #[test]
    fn automation_template_is_versioned_bounded_and_has_no_side_effect_executor() {
        assert!(AUTOMATION_SERVICE.contains("#[serde(deny_unknown_fields)]"));
        assert!(AUTOMATION_SERVICE.contains("execution_key: format!"));
        assert!(AUTOMATION_SERVICE.contains("UnsupportedAction"));
        assert!(!AUTOMATION_SERVICE.contains("DELETE FROM"));
        assert!(!AUTOMATION_SERVICE.contains("UPDATE achievements"));
    }
}
