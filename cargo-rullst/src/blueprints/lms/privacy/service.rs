pub(super) const PRIVACY_SERVICE: &str = r##"use crate::services::school_service;
use rullst_security::UserContext;

const MAX_RETENTION_SECONDS: i64 = 10 * 365 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeBand {
    Adult,
    Minor,
}

impl AgeBand {
    fn as_str(self) -> &'static str {
        match self { Self::Adult => "adult", Self::Minor => "minor" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyRequestKind {
    Export,
    Delete,
}

impl PrivacyRequestKind {
    fn as_str(self) -> &'static str {
        match self { Self::Export => "export", Self::Delete => "delete" }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyReceipt {
    pub applied: bool,
    pub school_id: i32,
    pub subject_user_id: i32,
    pub status: String,
}

#[derive(Debug)]
pub enum PrivacyError {
    Forbidden,
    NotConfigured,
    ConsentRequired,
    RetentionExpired,
    IdempotencyConflict,
    InvalidField(&'static str),
    School(String),
    Database(rullst_orm::Error),
}

impl std::fmt::Display for PrivacyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("privacy access denied"),
            Self::NotConfigured => formatter.write_str("privacy policy is not configured"),
            Self::ConsentRequired => formatter.write_str("active guardian consent is required"),
            Self::RetentionExpired => formatter.write_str("privacy retention window expired"),
            Self::IdempotencyConflict => formatter.write_str("privacy idempotency conflict"),
            Self::InvalidField(field) => write!(formatter, "invalid privacy field: {field}"),
            Self::School(error) => write!(formatter, "privacy school boundary: {error}"),
            Self::Database(error) => write!(formatter, "privacy database error: {error}"),
        }
    }
}

impl std::error::Error for PrivacyError {}

impl From<rullst_orm::Error> for PrivacyError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
    })
}

fn actor_id(context: &UserContext) -> Result<i32, PrivacyError> {
    context.user_id.parse::<i32>().map_err(|_| PrivacyError::Forbidden)
}

fn can_administer(context: &UserContext) -> bool {
    context.has_role("admin") || context.has_role("school_owner")
}

async fn school_id(context: &UserContext) -> Result<i32, PrivacyError> {
    school_service::context_school_id(context)
        .await
        .map_err(|error| PrivacyError::School(error.to_string()))
}

async fn has_membership_at(
    school_id: i32,
    user_id: i32,
    observed_at_epoch: i64,
) -> Result<bool, PrivacyError> {
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT COUNT(*) FROM school_memberships WHERE school_id = $1 AND user_id = $2 AND status = $3 AND valid_from_epoch <= $4 AND (expires_at_epoch = 0 OR expires_at_epoch > $5)",
        _ => "SELECT COUNT(*) FROM school_memberships WHERE school_id = ? AND user_id = ? AND status = ? AND valid_from_epoch <= ? AND (expires_at_epoch = 0 OR expires_at_epoch > ?)",
    };
    let count = rullst::db::sqlx::query_scalar::<_, i64>(sql)
        .bind(school_id).bind(user_id).bind("active")
        .bind(observed_at_epoch).bind(observed_at_epoch)
        .fetch_one(rullst::db::Orm::pool()?).await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    Ok(count == 1)
}

pub async fn configure_subject_policy_at(
    context: &UserContext,
    policy_key: &str,
    subject_user_id: i32,
    age_band: AgeBand,
    policy_version: &str,
    retention_until_epoch: i64,
    observed_at_epoch: i64,
) -> Result<PrivacyReceipt, PrivacyError> {
    if !can_administer(context) { return Err(PrivacyError::Forbidden); }
    if subject_user_id <= 0 || observed_at_epoch <= 0
        || !valid_key(policy_key, 128) || !valid_key(policy_version, 64)
        || retention_until_epoch <= observed_at_epoch
        || retention_until_epoch - observed_at_epoch > MAX_RETENTION_SECONDS
    {
        return Err(PrivacyError::InvalidField("subject policy"));
    }
    let school_id = school_id(context).await?;
    if !has_membership_at(school_id, subject_user_id, observed_at_epoch).await? {
        return Err(PrivacyError::Forbidden);
    }
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let replay_sql = match driver {
        "postgres" => "SELECT school_id, subject_user_id, age_band, policy_version, retention_until_epoch, status FROM privacy_subject_policies WHERE policy_key = $1",
        _ => "SELECT school_id, subject_user_id, age_band, policy_version, retention_until_epoch, status FROM privacy_subject_policies WHERE policy_key = ?",
    };
    let replay = rullst::db::sqlx::query_as::<_, (i32, i32, String, String, i64, String)>(replay_sql)
        .bind(policy_key).fetch_optional(pool).await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    if let Some(row) = replay {
        if row == (school_id, subject_user_id, age_band.as_str().to_string(), policy_version.to_string(), retention_until_epoch, "active".to_string()) {
            return Ok(PrivacyReceipt { applied: false, school_id, subject_user_id, status: row.5 });
        }
        return Err(PrivacyError::IdempotencyConflict);
    }

    let mut transaction = pool.begin().await.map_err(|error| PrivacyError::Database(error.into()))?;
    let supersede_sql = match driver {
        "postgres" => "UPDATE privacy_subject_policies SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE school_id = $2 AND subject_user_id = $3 AND status = $4",
        _ => "UPDATE privacy_subject_policies SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE school_id = ? AND subject_user_id = ? AND status = ?",
    };
    rullst::db::sqlx::query(supersede_sql).bind("superseded").bind(school_id)
        .bind(subject_user_id).bind("active").execute(&mut *transaction).await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    let insert_sql = match driver {
        "postgres" => "INSERT INTO privacy_subject_policies (policy_key, school_id, subject_user_id, age_band, policy_version, retention_until_epoch, status, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO privacy_subject_policies (policy_key, school_id, subject_user_id, age_band, policy_version, retention_until_epoch, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(insert_sql).bind(policy_key).bind(school_id)
        .bind(subject_user_id).bind(age_band.as_str()).bind(policy_version)
        .bind(retention_until_epoch).bind("active").execute(&mut *transaction).await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    transaction.commit().await.map_err(|error| PrivacyError::Database(error.into()))?;
    Ok(PrivacyReceipt { applied: true, school_id, subject_user_id, status: "active".to_string() })
}

pub async fn record_guardian_consent_at(
    context: &UserContext,
    consent_key: &str,
    subject_user_id: i32,
    purpose: &str,
    policy_version: &str,
    observed_at_epoch: i64,
) -> Result<PrivacyReceipt, PrivacyError> {
    let guardian_user_id = actor_id(context)?;
    if !context.has_role("guardian") || guardian_user_id == subject_user_id
        || subject_user_id <= 0 || observed_at_epoch <= 0
        || !valid_key(consent_key, 128) || !valid_key(purpose, 64)
        || !valid_key(policy_version, 64)
    {
        return Err(PrivacyError::Forbidden);
    }
    let school_id = school_id(context).await?;
    if !has_membership_at(school_id, guardian_user_id, observed_at_epoch).await?
        || !has_membership_at(school_id, subject_user_id, observed_at_epoch).await?
    {
        return Err(PrivacyError::Forbidden);
    }
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let policy_sql = match driver {
        "postgres" => "SELECT COUNT(*) FROM privacy_subject_policies WHERE school_id = $1 AND subject_user_id = $2 AND age_band = $3 AND policy_version = $4 AND status = $5 AND retention_until_epoch > $6",
        _ => "SELECT COUNT(*) FROM privacy_subject_policies WHERE school_id = ? AND subject_user_id = ? AND age_band = ? AND policy_version = ? AND status = ? AND retention_until_epoch > ?",
    };
    let policy_count = rullst::db::sqlx::query_scalar::<_, i64>(policy_sql)
        .bind(school_id).bind(subject_user_id).bind("minor").bind(policy_version)
        .bind("active").bind(observed_at_epoch).fetch_one(pool).await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    if policy_count != 1 { return Err(PrivacyError::NotConfigured); }

    let replay_sql = match driver {
        "postgres" => "SELECT school_id, subject_user_id, guardian_user_id, purpose, policy_version, status FROM guardian_consents WHERE consent_key = $1",
        _ => "SELECT school_id, subject_user_id, guardian_user_id, purpose, policy_version, status FROM guardian_consents WHERE consent_key = ?",
    };
    if let Some(row) = rullst::db::sqlx::query_as::<_, (i32, i32, i32, String, String, String)>(replay_sql)
        .bind(consent_key).fetch_optional(pool).await
        .map_err(|error| PrivacyError::Database(error.into()))?
    {
        if row.0 == school_id && row.1 == subject_user_id && row.2 == guardian_user_id
            && row.3 == purpose && row.4 == policy_version
        {
            return Ok(PrivacyReceipt { applied: false, school_id, subject_user_id, status: row.5 });
        }
        return Err(PrivacyError::IdempotencyConflict);
    }
    let insert_sql = match driver {
        "postgres" => "INSERT INTO guardian_consents (consent_key, school_id, subject_user_id, guardian_user_id, purpose, policy_version, status, granted_at_epoch, revoked_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO guardian_consents (consent_key, school_id, subject_user_id, guardian_user_id, purpose, policy_version, status, granted_at_epoch, revoked_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(insert_sql).bind(consent_key).bind(school_id)
        .bind(subject_user_id).bind(guardian_user_id).bind(purpose).bind(policy_version)
        .bind("active").bind(observed_at_epoch).execute(pool).await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    Ok(PrivacyReceipt { applied: true, school_id, subject_user_id, status: "active".to_string() })
}

pub async fn authorize_subject_at(
    context: &UserContext,
    subject_user_id: i32,
    purpose: &str,
    observed_at_epoch: i64,
) -> Result<(), PrivacyError> {
    let actor = actor_id(context)?;
    if actor != subject_user_id && !can_administer(context) { return Err(PrivacyError::Forbidden); }
    if subject_user_id <= 0 || observed_at_epoch <= 0 || !valid_key(purpose, 64) {
        return Err(PrivacyError::InvalidField("authorization"));
    }
    let school_id = school_id(context).await?;
    if !has_membership_at(school_id, subject_user_id, observed_at_epoch).await? {
        return Err(PrivacyError::Forbidden);
    }
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT age_band, policy_version, retention_until_epoch FROM privacy_subject_policies WHERE school_id = $1 AND subject_user_id = $2 AND status = $3",
        _ => "SELECT age_band, policy_version, retention_until_epoch FROM privacy_subject_policies WHERE school_id = ? AND subject_user_id = ? AND status = ?",
    };
    let policies = rullst::db::sqlx::query_as::<_, (String, String, i64)>(sql)
        .bind(school_id).bind(subject_user_id).bind("active")
        .fetch_all(rullst::db::Orm::pool()?).await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    if policies.len() != 1 { return Err(PrivacyError::NotConfigured); }
    let policy = policies.first().ok_or(PrivacyError::NotConfigured)?;
    if policy.2 <= observed_at_epoch { return Err(PrivacyError::RetentionExpired); }
    if policy.0 == "adult" { return Ok(()); }
    if policy.0 != "minor" { return Err(PrivacyError::NotConfigured); }
    let consent_sql = match driver {
        "postgres" => "SELECT COUNT(*) FROM guardian_consents WHERE school_id = $1 AND subject_user_id = $2 AND purpose = $3 AND policy_version = $4 AND status = $5 AND granted_at_epoch <= $6 AND revoked_at_epoch = 0",
        _ => "SELECT COUNT(*) FROM guardian_consents WHERE school_id = ? AND subject_user_id = ? AND purpose = ? AND policy_version = ? AND status = ? AND granted_at_epoch <= ? AND revoked_at_epoch = 0",
    };
    let consent_count = rullst::db::sqlx::query_scalar::<_, i64>(consent_sql)
        .bind(school_id).bind(subject_user_id).bind(purpose).bind(&policy.1)
        .bind("active").bind(observed_at_epoch)
        .fetch_one(rullst::db::Orm::pool()?).await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    if consent_count == 1 { Ok(()) } else { Err(PrivacyError::ConsentRequired) }
}

pub async fn request_privacy_action_at(
    context: &UserContext,
    request_key: &str,
    subject_user_id: i32,
    kind: PrivacyRequestKind,
    observed_at_epoch: i64,
) -> Result<PrivacyReceipt, PrivacyError> {
    let actor = actor_id(context)?;
    if actor != subject_user_id && !can_administer(context) { return Err(PrivacyError::Forbidden); }
    if subject_user_id <= 0 || observed_at_epoch <= 0 || !valid_key(request_key, 128) {
        return Err(PrivacyError::InvalidField("privacy request"));
    }
    let school_id = school_id(context).await?;
    if !has_membership_at(school_id, subject_user_id, observed_at_epoch).await? {
        return Err(PrivacyError::Forbidden);
    }
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let replay_sql = match driver {
        "postgres" => "SELECT school_id, subject_user_id, requested_by_user_id, request_kind, status FROM privacy_requests WHERE request_key = $1",
        _ => "SELECT school_id, subject_user_id, requested_by_user_id, request_kind, status FROM privacy_requests WHERE request_key = ?",
    };
    if let Some(row) = rullst::db::sqlx::query_as::<_, (i32, i32, i32, String, String)>(replay_sql)
        .bind(request_key).fetch_optional(pool).await
        .map_err(|error| PrivacyError::Database(error.into()))?
    {
        if row.0 == school_id && row.1 == subject_user_id && row.2 == actor && row.3 == kind.as_str() {
            return Ok(PrivacyReceipt { applied: false, school_id, subject_user_id, status: row.4 });
        }
        return Err(PrivacyError::IdempotencyConflict);
    }
    let insert_sql = match driver {
        "postgres" => "INSERT INTO privacy_requests (request_key, school_id, subject_user_id, requested_by_user_id, request_kind, status, requested_at_epoch, completed_at_epoch, result_digest, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, 0, '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO privacy_requests (request_key, school_id, subject_user_id, requested_by_user_id, request_kind, status, requested_at_epoch, completed_at_epoch, result_digest, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(insert_sql).bind(request_key).bind(school_id)
        .bind(subject_user_id).bind(actor).bind(kind.as_str()).bind("pending")
        .bind(observed_at_epoch).execute(pool).await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    Ok(PrivacyReceipt { applied: true, school_id, subject_user_id, status: "pending".to_string() })
}

pub async fn revoke_guardian_consent_at(
    context: &UserContext,
    consent_key: &str,
    observed_at_epoch: i64,
) -> Result<bool, PrivacyError> {
    if observed_at_epoch <= 0 || !valid_key(consent_key, 128) {
        return Err(PrivacyError::InvalidField("consent revocation"));
    }
    let actor = actor_id(context)?;
    let school_id = school_id(context).await?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let select_sql = match driver {
        "postgres" => "SELECT school_id, guardian_user_id, status FROM guardian_consents WHERE consent_key = $1",
        _ => "SELECT school_id, guardian_user_id, status FROM guardian_consents WHERE consent_key = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (i32, i32, String)>(select_sql)
        .bind(consent_key).fetch_optional(pool).await
        .map_err(|error| PrivacyError::Database(error.into()))?
        .ok_or(PrivacyError::Forbidden)?;
    if row.0 != school_id || (row.1 != actor && !can_administer(context)) {
        return Err(PrivacyError::Forbidden);
    }
    if row.2 == "revoked" { return Ok(false); }
    let update_sql = match driver {
        "postgres" => "UPDATE guardian_consents SET status = $1, revoked_at_epoch = $2, updated_at = CURRENT_TIMESTAMP WHERE consent_key = $3 AND school_id = $4 AND status = $5",
        _ => "UPDATE guardian_consents SET status = ?, revoked_at_epoch = ?, updated_at = CURRENT_TIMESTAMP WHERE consent_key = ? AND school_id = ? AND status = ?",
    };
    let result = rullst::db::sqlx::query(update_sql).bind("revoked").bind(observed_at_epoch)
        .bind(consent_key).bind(school_id).bind("active").execute(pool).await
        .map_err(|error| PrivacyError::Database(error.into()))?;
    Ok(result.rows_affected() == 1)
}
"##;
