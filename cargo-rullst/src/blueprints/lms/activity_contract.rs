// Versioned server-side activity/attempt/result contract templates.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/services/activity_contract.rs",
        ACTIVITY_CONTRACT.to_string(),
    )]
}

const ACTIVITY_CONTRACT: &str = r##"use rullst_security::{RbacGuard, UserContext};

pub const ACTIVITY_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Quiz,
    Exercise,
    Game,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityAttempt {
    pub schema_version: i32,
    pub attempt_key: String,
    pub activity_id: i32,
    pub subject_user_id: i32,
    pub kind: ActivityKind,
    pub ruleset_version: String,
    pub started_at_epoch_seconds: i64,
    pub state_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityResult {
    pub schema_version: i32,
    pub attempt_key: String,
    pub activity_id: i32,
    pub subject_user_id: i32,
    pub ruleset_version: String,
    pub points: i32,
    pub max_score: i32,
    pub finished_at_epoch_seconds: i64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedActivityResult {
    pub actor_user_id: i32,
    pub attempt: ActivityAttempt,
    pub result: ActivityResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityContractError {
    Forbidden,
    InvalidIdentity,
    UnsupportedSchemaVersion(i32),
    InvalidField(&'static str),
}

impl std::fmt::Display for ActivityContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("activity access denied"),
            Self::InvalidIdentity => formatter.write_str("authenticated activity actor is invalid"),
            Self::UnsupportedSchemaVersion(version) => write!(formatter, "unsupported activity schema version: {version}"),
            Self::InvalidField(field) => write!(formatter, "invalid activity contract field: {field}"),
        }
    }
}

impl std::error::Error for ActivityContractError {}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

pub fn validate_activity_result(
    context: &UserContext,
    attempt: ActivityAttempt,
    result: ActivityResult,
) -> Result<ValidatedActivityResult, ActivityContractError> {
    for version in [attempt.schema_version, result.schema_version] {
        if version != ACTIVITY_SCHEMA_VERSION {
            return Err(ActivityContractError::UnsupportedSchemaVersion(version));
        }
    }
    let actor_user_id = context
        .user_id
        .parse::<i32>()
        .map_err(|_| ActivityContractError::InvalidIdentity)?;
    RbacGuard::authorize_owner_or_role(
        context,
        &attempt.subject_user_id.to_string(),
        "admin",
    )
    .map_err(|_| ActivityContractError::Forbidden)?;

    if attempt.activity_id <= 0
        || attempt.subject_user_id <= 0
        || attempt.attempt_key != result.attempt_key
        || attempt.activity_id != result.activity_id
        || attempt.subject_user_id != result.subject_user_id
        || attempt.ruleset_version != result.ruleset_version
        || !valid_key(&attempt.attempt_key, 128)
        || !valid_key(&attempt.ruleset_version, 64)
    {
        return Err(ActivityContractError::InvalidField("identity binding"));
    }
    if attempt.started_at_epoch_seconds <= 0
        || result.finished_at_epoch_seconds < attempt.started_at_epoch_seconds
    {
        return Err(ActivityContractError::InvalidField("time ordering"));
    }
    if result.points < 0
        || result.max_score <= 0
        || result.points > result.max_score
        || result.max_score > 1_000_000
    {
        return Err(ActivityContractError::InvalidField("score bounds"));
    }
    if result.evidence_sha256.len() != 64
        || !result
            .evidence_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ActivityContractError::InvalidField("evidence_sha256"));
    }
    if attempt.state_json.len() > 64 * 1024 {
        return Err(ActivityContractError::InvalidField("state_json"));
    }

    Ok(ValidatedActivityResult {
        actor_user_id,
        attempt,
        result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt() -> ActivityAttempt {
        ActivityAttempt {
            schema_version: ACTIVITY_SCHEMA_VERSION,
            attempt_key: "attempt-1".to_string(),
            activity_id: 2,
            subject_user_id: 7,
            kind: ActivityKind::Game,
            ruleset_version: "rules-v1".to_string(),
            started_at_epoch_seconds: 1_000,
            state_json: "{\"level\":1}".to_string(),
        }
    }

    fn result() -> ActivityResult {
        ActivityResult {
            schema_version: ACTIVITY_SCHEMA_VERSION,
            attempt_key: "attempt-1".to_string(),
            activity_id: 2,
            subject_user_id: 7,
            ruleset_version: "rules-v1".to_string(),
            points: 80,
            max_score: 100,
            finished_at_epoch_seconds: 1_100,
            evidence_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn attempt_result_and_evidence_are_bound_to_the_owner() {
        let owner = UserContext::new("7", vec!["student".to_string()]);
        let attacker = UserContext::new("8", vec!["student".to_string()]);
        assert!(validate_activity_result(&owner, attempt(), result()).is_ok());
        assert!(matches!(
            validate_activity_result(&attacker, attempt(), result()),
            Err(ActivityContractError::Forbidden)
        ));

        let mut tampered = result();
        tampered.points = 101;
        assert!(matches!(
            validate_activity_result(&owner, attempt(), tampered),
            Err(ActivityContractError::InvalidField("score bounds"))
        ));
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::ACTIVITY_CONTRACT;

    #[test]
    fn generated_contract_binds_attempt_result_rules_and_evidence() {
        for field in [
            "attempt_key",
            "ruleset_version",
            "state_json",
            "evidence_sha256",
            "finished_at_epoch_seconds",
        ] {
            assert!(ACTIVITY_CONTRACT.contains(field));
        }
    }
}
