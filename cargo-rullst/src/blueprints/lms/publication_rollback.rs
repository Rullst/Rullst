// Audited immutable course-publication rollback templates.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/models/publication_rollback.rs",
            PUBLICATION_ROLLBACK_MODEL.to_string(),
        ),
        (
            "src/services/publication_rollback_service.rs",
            PUBLICATION_ROLLBACK_SERVICE.to_string(),
        ),
        (
            "src/controllers/publication_rollback_controller.rs",
            PUBLICATION_ROLLBACK_CONTROLLER.to_string(),
        ),
        (
            "src/migrations/m20260904000000_add_publication_rollbacks.rs",
            PUBLICATION_ROLLBACK_MIGRATION.to_string(),
        ),
    ]
}

const PUBLICATION_ROLLBACK_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "course_publication_rollbacks")]
pub struct PublicationRollback {
    pub id: i32,
    pub rollback_key: String,
    pub course_id: i32,
    pub source_version_id: i32,
    pub replaced_version_id: i32,
    pub result_version_id: i32,
    pub actor_user_id: i32,
    pub reason: String,
    pub occurred_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for PublicationRollback {
    fn nexus_table() -> &'static str { "course_publication_rollbacks" }
    fn nexus_label() -> &'static str { "Publication Rollbacks" }
    fn nexus_icon() -> &'static str { "↩️" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "rollback_key", label: "Rollback Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "source_version_id", label: "Restored Source", kind: FieldKind::ForeignKey { table: "course_versions", label_col: "version_key" }, hidden: false, readonly: true },
            FieldMeta { name: "replaced_version_id", label: "Replaced Version", kind: FieldKind::ForeignKey { table: "course_versions", label_col: "version_key" }, hidden: false, readonly: true },
            FieldMeta { name: "result_version_id", label: "New Published Version", kind: FieldKind::ForeignKey { table: "course_versions", label_col: "version_key" }, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Administrator", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "reason", label: "Reason", kind: FieldKind::Textarea, hidden: false, readonly: true },
            FieldMeta { name: "occurred_at_epoch", label: "Occurred Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
        ]
    }
}
"##;

const PUBLICATION_ROLLBACK_SERVICE: &str = r##"use crate::services::school_service;
use rullst_security::{RbacGuard, UserContext};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PublicationRollbackReceipt {
    pub rollback_id: i32,
    pub rollback_key: String,
    pub course_id: i32,
    pub source_version_id: i32,
    pub replaced_version_id: i32,
    pub result_version_id: i32,
    pub result_version_key: String,
    pub result_revision: i32,
    pub applied: bool,
}

#[derive(Debug)]
pub enum PublicationRollbackError {
    Forbidden,
    NotFound,
    InvalidField(&'static str),
    InvalidState,
    SeparationOfDuties,
    IdempotencyConflict,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for PublicationRollbackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("course rollback access denied"),
            Self::NotFound => formatter.write_str("course rollback resource not found"),
            Self::InvalidField(field) => write!(formatter, "invalid course rollback field: {field}"),
            Self::InvalidState => formatter.write_str("course rollback state rejected"),
            Self::SeparationOfDuties => formatter.write_str("course author cannot approve rollback of the same snapshot"),
            Self::IdempotencyConflict => formatter.write_str("course rollback idempotency conflict"),
            Self::Database(error) => write!(formatter, "course rollback database error: {error}"),
        }
    }
}

impl std::error::Error for PublicationRollbackError {}

impl From<rullst_orm::Error> for PublicationRollbackError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

fn actor_id(context: &UserContext) -> Result<i32, PublicationRollbackError> {
    RbacGuard::authorize(context, "admin").map_err(|_| PublicationRollbackError::Forbidden)?;
    context.user_id.parse::<i32>().ok().filter(|id| *id > 0)
        .ok_or(PublicationRollbackError::Forbidden)
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
        })
}

fn validate_reason(reason: &str) -> Result<&str, PublicationRollbackError> {
    let reason = reason.trim();
    if !(8..=512).contains(&reason.len()) || reason.chars().any(char::is_control) {
        return Err(PublicationRollbackError::InvalidField("reason"));
    }
    Ok(reason)
}

pub async fn rollback_course_at(
    context: &UserContext,
    course_id: i32,
    source_version_id: i32,
    rollback_key: &str,
    reason: &str,
    now_epoch: i64,
) -> Result<PublicationRollbackReceipt, PublicationRollbackError> {
    let actor_user_id = actor_id(context)?;
    let reason = validate_reason(reason)?;
    if course_id <= 0 || source_version_id <= 0 || now_epoch <= 0 || !valid_key(rollback_key, 64) {
        return Err(PublicationRollbackError::InvalidField("request"));
    }
    school_service::authorize_course(context, course_id).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => PublicationRollbackError::Database(error),
            _ => PublicationRollbackError::Forbidden,
        })?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool.begin().await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?;

    let current_sql = match driver {
        "postgres" | "mysql" => "SELECT id FROM course_versions WHERE course_id = ? AND status = ? ORDER BY id ASC FOR UPDATE",
        _ => "SELECT id FROM course_versions WHERE course_id = ? AND status = ? ORDER BY id ASC",
    };
    let current_sql = if driver == "postgres" {
        "SELECT id FROM course_versions WHERE course_id = $1 AND status = $2 ORDER BY id ASC FOR UPDATE"
    } else { current_sql };
    let current = rullst::db::sqlx::query_scalar::<_, i32>(current_sql)
        .bind(course_id).bind("published").fetch_all(&mut *transaction).await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?;
    if current.len() != 1 { return Err(PublicationRollbackError::InvalidState); }
    let replaced_version_id = current.into_iter().next()
        .ok_or(PublicationRollbackError::InvalidState)?;

    let existing_sql = match driver {
        "postgres" => "SELECT id, course_id, source_version_id, replaced_version_id, result_version_id, actor_user_id, reason FROM course_publication_rollbacks WHERE rollback_key = $1",
        _ => "SELECT id, course_id, source_version_id, replaced_version_id, result_version_id, actor_user_id, reason FROM course_publication_rollbacks WHERE rollback_key = ?",
    };
    if let Some(existing) = rullst::db::sqlx::query_as::<_, (i32, i32, i32, i32, i32, i32, String)>(existing_sql)
        .bind(rollback_key).fetch_optional(&mut *transaction).await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?
    {
        if existing.1 != course_id || existing.2 != source_version_id
            || existing.5 != actor_user_id || existing.6 != reason
        {
            return Err(PublicationRollbackError::IdempotencyConflict);
        }
        let result_sql = match driver {
            "postgres" => "SELECT version_key, revision FROM course_versions WHERE id = $1",
            _ => "SELECT version_key, revision FROM course_versions WHERE id = ?",
        };
        let result = rullst::db::sqlx::query_as::<_, (String, i32)>(result_sql)
            .bind(existing.4).fetch_optional(&mut *transaction).await
            .map_err(|error| PublicationRollbackError::Database(error.into()))?
            .ok_or(PublicationRollbackError::InvalidState)?;
        transaction.commit().await
            .map_err(|error| PublicationRollbackError::Database(error.into()))?;
        return Ok(PublicationRollbackReceipt {
            rollback_id: existing.0, rollback_key: rollback_key.to_string(), course_id,
            source_version_id, replaced_version_id: existing.3, result_version_id: existing.4,
            result_version_key: result.0, result_revision: result.1, applied: false,
        });
    }

    if replaced_version_id == source_version_id {
        return Err(PublicationRollbackError::InvalidState);
    }
    let source_sql = match driver {
        "postgres" => "SELECT content_json, authored_by, status FROM course_versions WHERE id = $1 AND course_id = $2",
        _ => "SELECT content_json, authored_by, status FROM course_versions WHERE id = ? AND course_id = ?",
    };
    let source = rullst::db::sqlx::query_as::<_, (String, i32, String)>(source_sql)
        .bind(source_version_id).bind(course_id).fetch_optional(&mut *transaction).await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?
        .ok_or(PublicationRollbackError::NotFound)?;
    if !matches!(source.2.as_str(), "published" | "archived") {
        return Err(PublicationRollbackError::InvalidState);
    }
    if source.1 == actor_user_id { return Err(PublicationRollbackError::SeparationOfDuties); }

    let revision_sql = match driver {
        "postgres" => "SELECT COALESCE(MAX(revision), 0) FROM course_versions WHERE course_id = $1",
        _ => "SELECT COALESCE(MAX(revision), 0) FROM course_versions WHERE course_id = ?",
    };
    let prior_revision = rullst::db::sqlx::query_scalar::<_, i32>(revision_sql)
        .bind(course_id).fetch_one(&mut *transaction).await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?;
    let result_revision = prior_revision.checked_add(1)
        .ok_or(PublicationRollbackError::InvalidField("revision"))?;
    let result_version_key = format!("rollback:{course_id}:{rollback_key}");
    if !valid_key(&result_version_key, 96) {
        return Err(PublicationRollbackError::InvalidField("result_version_key"));
    }

    let archive_sql = match driver {
        "postgres" => "UPDATE course_versions SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2 AND course_id = $3 AND status = $4",
        _ => "UPDATE course_versions SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND course_id = ? AND status = ?",
    };
    let archived = rullst::db::sqlx::query(archive_sql).bind("archived")
        .bind(replaced_version_id).bind(course_id).bind("published")
        .execute(&mut *transaction).await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?.rows_affected();
    if archived != 1 { return Err(PublicationRollbackError::InvalidState); }

    let insert_version_sql = match driver {
        "postgres" => "INSERT INTO course_versions (course_id, version_key, revision, status, content_json, authored_by, reviewed_by, scheduled_at_epoch, published_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO course_versions (course_id, version_key, revision, status, content_json, authored_by, reviewed_by, scheduled_at_epoch, published_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(insert_version_sql).bind(course_id).bind(&result_version_key)
        .bind(result_revision).bind("published").bind(&source.0).bind(source.1)
        .bind(actor_user_id).bind(now_epoch).execute(&mut *transaction).await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?;
    let result_id_sql = match driver {
        "postgres" => "SELECT id FROM course_versions WHERE version_key = $1",
        _ => "SELECT id FROM course_versions WHERE version_key = ?",
    };
    let result_version_id = rullst::db::sqlx::query_scalar::<_, i32>(result_id_sql)
        .bind(&result_version_key).fetch_one(&mut *transaction).await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?;

    let audit_sql = match driver {
        "postgres" => "INSERT INTO course_publication_rollbacks (rollback_key, course_id, source_version_id, replaced_version_id, result_version_id, actor_user_id, reason, occurred_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO course_publication_rollbacks (rollback_key, course_id, source_version_id, replaced_version_id, result_version_id, actor_user_id, reason, occurred_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(audit_sql).bind(rollback_key).bind(course_id)
        .bind(source_version_id).bind(replaced_version_id).bind(result_version_id)
        .bind(actor_user_id).bind(reason).bind(now_epoch).execute(&mut *transaction).await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?;
    let audit_id_sql = match driver {
        "postgres" => "SELECT id FROM course_publication_rollbacks WHERE rollback_key = $1",
        _ => "SELECT id FROM course_publication_rollbacks WHERE rollback_key = ?",
    };
    let rollback_id = rullst::db::sqlx::query_scalar::<_, i32>(audit_id_sql)
        .bind(rollback_key).fetch_one(&mut *transaction).await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?;

    let event_key = format!("course-rolled-back:{rollback_key}");
    let payload = serde_json::json!({
        "schema_version": 1, "actor_user_id": actor_user_id, "course_id": course_id,
        "source_version_id": source_version_id, "replaced_version_id": replaced_version_id,
        "result_version_id": result_version_id, "rollback_key": rollback_key, "reason": reason,
    }).to_string();
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => PublicationRollbackError::Database(error),
            _ => PublicationRollbackError::Forbidden,
        })?;
    let event_sql = match driver {
        "postgres" => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, 0, $4, $5, 0, $6, $7, $8, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, 0, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, 0, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    rullst::db::sqlx::query(event_sql).bind(school_id).bind(event_key).bind("course_rolled_back")
        .bind(payload).bind("pending").bind("").bind("").bind("")
        .execute(&mut *transaction).await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?;
    transaction.commit().await
        .map_err(|error| PublicationRollbackError::Database(error.into()))?;
    Ok(PublicationRollbackReceipt {
        rollback_id, rollback_key: rollback_key.to_string(), course_id, source_version_id,
        replaced_version_id, result_version_id, result_version_key, result_revision, applied: true,
    })
}
"##;

const PUBLICATION_ROLLBACK_CONTROLLER: &str = r##"use crate::services::publication_rollback_service::{
    PublicationRollbackError, rollback_course_at,
};
use rullst::server::{Extension, IntoResponse, Json, Path, Response, StatusCode};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PublicationRollbackPayload {
    pub source_version_id: i32,
    pub rollback_key: String,
    pub reason: String,
}

fn error_response(error: PublicationRollbackError) -> Response {
    match error {
        PublicationRollbackError::Forbidden => StatusCode::FORBIDDEN.into_response(),
        PublicationRollbackError::NotFound => StatusCode::NOT_FOUND.into_response(),
        PublicationRollbackError::InvalidField(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        PublicationRollbackError::InvalidState
        | PublicationRollbackError::SeparationOfDuties
        | PublicationRollbackError::IdempotencyConflict => StatusCode::CONFLICT.into_response(),
        PublicationRollbackError::Database(error) => {
            eprintln!("Course rollback operation failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

fn unix_now() -> Result<i64, Response> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub async fn rollback(
    Path(course_id): Path<i32>,
    Extension(context): Extension<UserContext>,
    Json(payload): Json<PublicationRollbackPayload>,
) -> Response {
    let now_epoch = match unix_now() {
        Ok(value) => value,
        Err(response) => return response,
    };
    match rollback_course_at(
        &context, course_id, payload.source_version_id, &payload.rollback_key,
        &payload.reason, now_epoch,
    ).await {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => error_response(error),
    }
}
"##;

const PUBLICATION_ROLLBACK_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m20260904000000_add_publication_rollbacks" }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("course_publication_rollbacks", |table| {
            table.id(); table.string("rollback_key").not_null(); table.integer("course_id").not_null();
            table.integer("source_version_id").not_null(); table.integer("replaced_version_id").not_null();
            table.integer("result_version_id").not_null(); table.integer("actor_user_id").not_null();
            table.string("reason").not_null(); table.big_integer("occurred_at_epoch").not_null();
            table.timestamps();
        }).await?;
        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX course_publication_rollbacks_key_unique ON course_publication_rollbacks(rollback_key)",
            "CREATE UNIQUE INDEX course_publication_rollbacks_result_unique ON course_publication_rollbacks(result_version_id)",
            "CREATE INDEX course_publication_rollbacks_course_time_idx ON course_publication_rollbacks(course_id, occurred_at_epoch)",
        ] { sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?; }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("course_publication_rollbacks").await
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::{
        PUBLICATION_ROLLBACK_CONTROLLER, PUBLICATION_ROLLBACK_MIGRATION,
        PUBLICATION_ROLLBACK_SERVICE,
    };

    #[test]
    fn rollback_creates_a_new_snapshot_and_durable_audit() {
        assert!(PUBLICATION_ROLLBACK_SERVICE.contains("result_version_key"));
        assert!(PUBLICATION_ROLLBACK_SERVICE.contains("course_rolled_back"));
        assert!(PUBLICATION_ROLLBACK_SERVICE.contains("IdempotencyConflict"));
        assert!(PUBLICATION_ROLLBACK_SERVICE.contains("FOR UPDATE"));
        assert!(PUBLICATION_ROLLBACK_MIGRATION.contains("rollbacks_key_unique"));
        assert!(PUBLICATION_ROLLBACK_CONTROLLER.contains("SystemTime::now"));
        assert!(!PUBLICATION_ROLLBACK_CONTROLLER.contains("pub actor_user_id"));
        assert!(!PUBLICATION_ROLLBACK_SERVICE.contains("format!(\"SELECT"));
    }
}
