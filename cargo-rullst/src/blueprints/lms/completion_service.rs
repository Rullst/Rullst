// Server-derived course completion, public verification and audited revocation.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/services/completion_service.rs",
        COMPLETION_SERVICE.to_string(),
    )]
}

const COMPLETION_SERVICE: &str = r##"use crate::services::school_service;
use rullst_security::{RbacGuard, UserContext};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CompletionReceipt {
    pub completion_id: i32,
    pub certificate_key: String,
    pub course_id: i32,
    pub version_key: String,
    pub ruleset_version: String,
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CertificateVerification {
    pub certificate_key: String,
    pub valid: bool,
    pub course_id: i32,
    pub version_key: String,
    pub ruleset_version: String,
    pub issued_at_epoch: i64,
    pub revoked_at_epoch: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CertificateRevocationReceipt {
    pub certificate_key: String,
    pub status: String,
    pub applied: bool,
}

#[derive(Debug)]
pub enum CompletionError {
    Forbidden,
    NotFound,
    Incomplete,
    InvalidField(&'static str),
    InvalidState,
    IdempotencyConflict,
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for CompletionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("course completion access denied"),
            Self::NotFound => formatter.write_str("course completion resource not found"),
            Self::Incomplete => formatter.write_str("versioned course requirements are incomplete"),
            Self::InvalidField(field) => write!(formatter, "invalid course completion field: {field}"),
            Self::InvalidState => formatter.write_str("certificate state transition rejected"),
            Self::IdempotencyConflict => formatter.write_str("certificate revocation key conflicts"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "course completion database error: {error}"),
        }
    }
}

impl std::error::Error for CompletionError {}

impl From<rullst_orm::Error> for CompletionError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

#[derive(Debug, Deserialize)]
struct CourseSnapshot {
    schema_version: i32,
    completion: CompletionRule,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompletionRule {
    schema_version: i32,
    ruleset_version: String,
    required_lesson_ids: Vec<i32>,
    required_progress_percent: i32,
}

pub async fn derive_completion(
    context: &UserContext,
    subject_user_id: i32,
    course_id: i32,
) -> Result<CompletionReceipt, CompletionError> {
    derive_completion_at(context, subject_user_id, course_id, unix_now()?).await
}

pub async fn derive_completion_at(
    context: &UserContext,
    subject_user_id: i32,
    course_id: i32,
    completed_at_epoch: i64,
) -> Result<CompletionReceipt, CompletionError> {
    authorize_owner(context, subject_user_id)?;
    if subject_user_id <= 0 || course_id <= 0 || completed_at_epoch <= 0 {
        return Err(CompletionError::InvalidField("completion request"));
    }
    let school_id = school_service::authorize_course_enrollment_at(
        context,
        subject_user_id,
        course_id,
        completed_at_epoch,
    ).await.map_err(|error| match error {
        school_service::SchoolError::Database(error) => CompletionError::Database(error),
        _ => CompletionError::Forbidden,
    })?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool.begin().await
        .map_err(|error| CompletionError::Database(error.into()))?;
    let pin_sql = match driver {
        "postgres" => "SELECT e.id, p.course_version_id FROM enrollments e INNER JOIN enrollment_content_versions p ON p.enrollment_id = e.id WHERE e.user_id = $1 AND e.course_id = $2 AND e.status = $3",
        _ => "SELECT e.id, p.course_version_id FROM enrollments e INNER JOIN enrollment_content_versions p ON p.enrollment_id = e.id WHERE e.user_id = ? AND e.course_id = ? AND e.status = ?",
    };
    let (_, course_version_id) = rullst::db::sqlx::query_as::<_, (i32, i32)>(pin_sql)
        .bind(subject_user_id).bind(course_id).bind("active")
        .fetch_optional(&mut *transaction).await
        .map_err(|error| CompletionError::Database(error.into()))?
        .ok_or(CompletionError::Forbidden)?;
    let version_sql = match driver {
        "postgres" => "SELECT version_key, status, content_json FROM course_versions WHERE id = $1 AND course_id = $2",
        _ => "SELECT version_key, status, content_json FROM course_versions WHERE id = ? AND course_id = ?",
    };
    let (version_key, version_status, content_json) =
        rullst::db::sqlx::query_as::<_, (String, String, String)>(version_sql)
            .bind(course_version_id).bind(course_id)
            .fetch_optional(&mut *transaction).await
            .map_err(|error| CompletionError::Database(error.into()))?
            .ok_or(CompletionError::NotFound)?;
    if !matches!(version_status.as_str(), "published" | "archived") {
        return Err(CompletionError::InvalidState);
    }
    let snapshot = serde_json::from_str::<CourseSnapshot>(&content_json)
        .map_err(|_| CompletionError::InvalidField("completion ruleset"))?;
    validate_rule(&snapshot)?;
    let completion_key = format!("course-completion:{subject_user_id}:{course_version_id}");
    let replay_sql = match driver {
        "postgres" => "SELECT cc.id, c.certificate_key, cv.version_key, cc.ruleset_version FROM course_completions cc INNER JOIN certificates c ON c.completion_id = cc.id INNER JOIN course_versions cv ON cv.id = cc.course_version_id WHERE cc.completion_key = $1",
        _ => "SELECT cc.id, c.certificate_key, cv.version_key, cc.ruleset_version FROM course_completions cc INNER JOIN certificates c ON c.completion_id = cc.id INNER JOIN course_versions cv ON cv.id = cc.course_version_id WHERE cc.completion_key = ?",
    };
    if let Some(row) = rullst::db::sqlx::query_as::<_, (i32, String, String, String)>(replay_sql)
        .bind(&completion_key).fetch_optional(&mut *transaction).await
        .map_err(|error| CompletionError::Database(error.into()))?
    {
        transaction.commit().await
            .map_err(|error| CompletionError::Database(error.into()))?;
        return Ok(receipt_from_row(row, course_id, false));
    }
    let progress_sql = match driver {
        "postgres" => "SELECT p.lesson_id, p.progress_percent FROM lesson_progress p INNER JOIN lessons l ON l.id = p.lesson_id WHERE p.user_id = $1 AND l.course_id = $2",
        _ => "SELECT p.lesson_id, p.progress_percent FROM lesson_progress p INNER JOIN lessons l ON l.id = p.lesson_id WHERE p.user_id = ? AND l.course_id = ?",
    };
    let progress = rullst::db::sqlx::query_as::<_, (i32, i32)>(progress_sql)
        .bind(subject_user_id).bind(course_id)
        .fetch_all(&mut *transaction).await
        .map_err(|error| CompletionError::Database(error.into()))?;
    if !snapshot.completion.required_lesson_ids.iter().all(|lesson_id| {
        progress.iter().any(|(observed_lesson_id, percent)| {
            observed_lesson_id == lesson_id
                && *percent >= snapshot.completion.required_progress_percent
        })
    }) {
        return Err(CompletionError::Incomplete);
    }
    let evidence_json = serde_json::json!({
        "schema_version": 1,
        "course_version_id": course_version_id,
        "version_key": version_key,
        "ruleset_version": snapshot.completion.ruleset_version,
        "required_lesson_ids": snapshot.completion.required_lesson_ids,
        "required_progress_percent": snapshot.completion.required_progress_percent,
    }).to_string();
    let insert_sql = match driver {
        "postgres" => "INSERT INTO course_completions (completion_key, subject_user_id, course_id, course_version_id, ruleset_version, completed_at_epoch, evidence_json, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO course_completions (completion_key, subject_user_id, course_id, course_version_id, ruleset_version, completed_at_epoch, evidence_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO course_completions (completion_key, subject_user_id, course_id, course_version_id, ruleset_version, completed_at_epoch, evidence_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    let inserted = rullst::db::sqlx::query(insert_sql)
        .bind(&completion_key).bind(subject_user_id).bind(course_id).bind(course_version_id)
        .bind(&snapshot.completion.ruleset_version).bind(completed_at_epoch).bind(evidence_json)
        .execute(&mut *transaction).await
        .map_err(|error| CompletionError::Database(error.into()))?.rows_affected() == 1;
    if !inserted {
        let row = rullst::db::sqlx::query_as::<_, (i32, String, String, String)>(replay_sql)
            .bind(&completion_key).fetch_one(&mut *transaction).await
            .map_err(|error| CompletionError::Database(error.into()))?;
        transaction.commit().await
            .map_err(|error| CompletionError::Database(error.into()))?;
        return Ok(receipt_from_row(row, course_id, false));
    }
    let completion_id_sql = match driver {
        "postgres" => "SELECT id FROM course_completions WHERE completion_key = $1",
        _ => "SELECT id FROM course_completions WHERE completion_key = ?",
    };
    let completion_id = rullst::db::sqlx::query_scalar::<_, i32>(completion_id_sql)
        .bind(&completion_key).fetch_one(&mut *transaction).await
        .map_err(|error| CompletionError::Database(error.into()))?;
    let certificate_key = format!("cert_{}", rullst::security::generate_csrf_token());
    let certificate_sql = match driver {
        "postgres" => "INSERT INTO certificates (certificate_key, completion_id, status, issued_at_epoch, revocation_key, revoked_by, revoked_at_epoch, revocation_reason, created_at, updated_at) VALUES ($1, $2, $3, $4, NULL, 0, 0, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO certificates (certificate_key, completion_id, status, issued_at_epoch, revocation_key, revoked_by, revoked_at_epoch, revocation_reason, created_at, updated_at) VALUES (?, ?, ?, ?, NULL, 0, 0, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(certificate_sql).bind(&certificate_key).bind(completion_id)
        .bind("valid").bind(completed_at_epoch).bind("")
        .execute(&mut *transaction).await
        .map_err(|error| CompletionError::Database(error.into()))?;
    let actor_user_id = actor_id(context)?;
    let event_key = format!("course-completed:{subject_user_id}:{course_version_id}");
    let payload = serde_json::json!({"schema_version":1,"actor_user_id":actor_user_id,"subject_user_id":subject_user_id,"course_id":course_id,"course_version_id":course_version_id,"completion_id":completion_id,"ruleset_version":snapshot.completion.ruleset_version}).to_string();
    insert_outbox(&mut transaction, driver, school_id, &event_key, "course_completed", subject_user_id, &payload).await?;
    transaction.commit().await
        .map_err(|error| CompletionError::Database(error.into()))?;
    Ok(CompletionReceipt { completion_id, certificate_key, course_id, version_key, ruleset_version: snapshot.completion.ruleset_version, applied: true })
}

pub async fn verify_certificate(
    certificate_key: &str,
) -> Result<CertificateVerification, CompletionError> {
    if !valid_key(certificate_key, 64) { return Err(CompletionError::InvalidField("certificate key")); }
    let sql = match rullst::db::Orm::driver()? {
        "postgres" => "SELECT c.status, cc.course_id, cv.version_key, cc.ruleset_version, c.issued_at_epoch, c.revoked_at_epoch FROM certificates c INNER JOIN course_completions cc ON cc.id = c.completion_id INNER JOIN course_versions cv ON cv.id = cc.course_version_id WHERE c.certificate_key = $1",
        _ => "SELECT c.status, cc.course_id, cv.version_key, cc.ruleset_version, c.issued_at_epoch, c.revoked_at_epoch FROM certificates c INNER JOIN course_completions cc ON cc.id = c.completion_id INNER JOIN course_versions cv ON cv.id = cc.course_version_id WHERE c.certificate_key = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (String, i32, String, String, i64, i64)>(sql)
        .bind(certificate_key).fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| CompletionError::Database(error.into()))?
        .ok_or(CompletionError::NotFound)?;
    Ok(CertificateVerification { certificate_key: certificate_key.to_string(), valid: row.0 == "valid", course_id: row.1, version_key: row.2, ruleset_version: row.3, issued_at_epoch: row.4, revoked_at_epoch: row.5 })
}

pub async fn revoke_certificate_at(
    context: &UserContext,
    revocation_key: &str,
    certificate_key: &str,
    revoked_at_epoch: i64,
    reason: &str,
) -> Result<CertificateRevocationReceipt, CompletionError> {
    RbacGuard::authorize(context, "admin").map_err(|_| CompletionError::Forbidden)?;
    let actor_user_id = actor_id(context)?;
    if !valid_key(revocation_key, 96) || !valid_key(certificate_key, 64)
        || revoked_at_epoch <= 0 || !(8..=256).contains(&reason.len())
        || reason.chars().any(char::is_control)
    { return Err(CompletionError::InvalidField("certificate revocation")); }
    let school_id = authorize_certificate_scope(context, certificate_key).await?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool.begin().await
        .map_err(|error| CompletionError::Database(error.into()))?;
    let fetch_sql = match driver {
        "postgres" => "SELECT c.id, c.status, c.revocation_key, c.revoked_by, c.revocation_reason, cc.subject_user_id, cc.course_id FROM certificates c INNER JOIN course_completions cc ON cc.id = c.completion_id WHERE c.certificate_key = $1",
        _ => "SELECT c.id, c.status, c.revocation_key, c.revoked_by, c.revocation_reason, cc.subject_user_id, cc.course_id FROM certificates c INNER JOIN course_completions cc ON cc.id = c.completion_id WHERE c.certificate_key = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (i32, String, Option<String>, i32, String, i32, i32)>(fetch_sql)
        .bind(certificate_key).fetch_optional(&mut *transaction).await
        .map_err(|error| CompletionError::Database(error.into()))?
        .ok_or(CompletionError::NotFound)?;
    if row.1 == "revoked" {
        if row.2.as_deref() == Some(revocation_key) && row.3 == actor_user_id && row.4 == reason {
            transaction.commit().await
                .map_err(|error| CompletionError::Database(error.into()))?;
            return Ok(CertificateRevocationReceipt { certificate_key: certificate_key.to_string(), status: "revoked".to_string(), applied: false });
        }
        return Err(CompletionError::IdempotencyConflict);
    }
    if row.1 != "valid" { return Err(CompletionError::InvalidState); }
    let conflict_sql = match driver {
        "postgres" => "SELECT id FROM certificates WHERE revocation_key = $1 AND id <> $2",
        _ => "SELECT id FROM certificates WHERE revocation_key = ? AND id <> ?",
    };
    if rullst::db::sqlx::query_scalar::<_, i32>(conflict_sql).bind(revocation_key).bind(row.0)
        .fetch_optional(&mut *transaction).await
        .map_err(|error| CompletionError::Database(error.into()))?.is_some()
    { return Err(CompletionError::IdempotencyConflict); }
    let update_sql = match driver {
        "postgres" => "UPDATE certificates SET status = $1, revocation_key = $2, revoked_by = $3, revoked_at_epoch = $4, revocation_reason = $5, updated_at = CURRENT_TIMESTAMP WHERE id = $6 AND status = $7",
        _ => "UPDATE certificates SET status = ?, revocation_key = ?, revoked_by = ?, revoked_at_epoch = ?, revocation_reason = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = ?",
    };
    let changed = rullst::db::sqlx::query(update_sql).bind("revoked").bind(revocation_key)
        .bind(actor_user_id).bind(revoked_at_epoch).bind(reason).bind(row.0).bind("valid")
        .execute(&mut *transaction).await
        .map_err(|error| CompletionError::Database(error.into()))?.rows_affected() == 1;
    if !changed { return Err(CompletionError::InvalidState); }
    let event_key = format!("certificate-revoked:{revocation_key}");
    let payload = serde_json::json!({"schema_version":1,"actor_user_id":actor_user_id,"subject_user_id":row.5,"course_id":row.6,"certificate_key":certificate_key,"reason":reason}).to_string();
    insert_outbox(&mut transaction, driver, school_id, &event_key, "certificate_revoked", row.5, &payload).await?;
    transaction.commit().await
        .map_err(|error| CompletionError::Database(error.into()))?;
    Ok(CertificateRevocationReceipt { certificate_key: certificate_key.to_string(), status: "revoked".to_string(), applied: true })
}

async fn authorize_certificate_scope(
    context: &UserContext,
    certificate_key: &str,
) -> Result<i32, CompletionError> {
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT cc.course_id FROM certificates c INNER JOIN course_completions cc ON cc.id = c.completion_id WHERE c.certificate_key = $1",
        _ => "SELECT cc.course_id FROM certificates c INNER JOIN course_completions cc ON cc.id = c.completion_id WHERE c.certificate_key = ?",
    };
    let course_id = rullst::db::sqlx::query_scalar::<_, i32>(sql).bind(certificate_key)
        .fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| CompletionError::Database(error.into()))?
        .ok_or(CompletionError::NotFound)?;
    school_service::authorize_course(context, course_id).await
        .map(|(school_id, _)| school_id)
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => CompletionError::Database(error),
            _ => CompletionError::Forbidden,
        })
}

async fn insert_outbox(
    transaction: &mut rullst::db::sqlx::Transaction<'_, rullst_orm::RullstDatabase>,
    driver: &str,
    school_id: i32,
    event_key: &str,
    event_kind: &str,
    subject_user_id: i32,
    payload: &str,
) -> Result<(), CompletionError> {
    let sql = match driver {
        "postgres" => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    rullst::db::sqlx::query(sql).bind(school_id).bind(event_key).bind(event_kind).bind(subject_user_id)
        .bind(payload).bind("pending").bind("").bind("").bind("")
        .execute(&mut **transaction).await
        .map_err(|error| CompletionError::Database(error.into()))?;
    Ok(())
}

fn receipt_from_row(row: (i32, String, String, String), course_id: i32, applied: bool) -> CompletionReceipt {
    CompletionReceipt { completion_id: row.0, certificate_key: row.1, course_id, version_key: row.2, ruleset_version: row.3, applied }
}

fn validate_rule(snapshot: &CourseSnapshot) -> Result<(), CompletionError> {
    let rule = &snapshot.completion;
    let unique = rule.required_lesson_ids.iter().copied().collect::<HashSet<_>>();
    if snapshot.schema_version != 1 || rule.schema_version != 1
        || !valid_key(&rule.ruleset_version, 96)
        || rule.required_lesson_ids.is_empty() || rule.required_lesson_ids.len() > 1_000
        || unique.len() != rule.required_lesson_ids.len()
        || unique.iter().any(|lesson_id| *lesson_id <= 0)
        || !(1..=100).contains(&rule.required_progress_percent)
    { return Err(CompletionError::InvalidField("completion ruleset")); }
    Ok(())
}

fn authorize_owner(context: &UserContext, subject_user_id: i32) -> Result<(), CompletionError> {
    RbacGuard::authorize_owner_or_role(context, &subject_user_id.to_string(), "admin")
        .map_err(|_| CompletionError::Forbidden)
}

fn actor_id(context: &UserContext) -> Result<i32, CompletionError> {
    context.user_id.parse::<i32>().ok().filter(|value| *value > 0)
        .ok_or(CompletionError::Forbidden)
}

fn unix_now() -> Result<i64, CompletionError> {
    let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| CompletionError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| CompletionError::Clock)
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
    })
}
"##;

#[cfg(test)]
mod tests {
    use super::COMPLETION_SERVICE;

    #[test]
    fn completion_is_snapshot_derived_idempotent_and_audited() {
        assert!(COMPLETION_SERVICE.contains("enrollment_content_versions"));
        assert!(COMPLETION_SERVICE.contains("required_lesson_ids"));
        assert!(!COMPLETION_SERVICE.contains("course_completions_subject_version_unique"));
        assert!(COMPLETION_SERVICE.contains("course_completed"));
        assert!(COMPLETION_SERVICE.contains("certificate_revoked"));
        assert!(COMPLETION_SERVICE.contains("authorize_owner_or_role"));
        assert!(!COMPLETION_SERVICE.contains("format!(\"SELECT"));
    }
}
