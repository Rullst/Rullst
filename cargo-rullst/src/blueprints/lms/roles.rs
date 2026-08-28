// Durable education-role assignment templates.

#[path = "role_controller.rs"]
mod role_controller;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = vec![
        (
            "src/models/role_assignment.rs",
            ROLE_ASSIGNMENT_MODEL.to_string(),
        ),
        ("src/services/role_service.rs", ROLE_SERVICE.to_string()),
        (
            "src/migrations/m20260902000000_add_education_roles.rs",
            ROLE_MIGRATION.to_string(),
        ),
    ];
    files.extend(role_controller::get_files());
    files
}

const ROLE_ASSIGNMENT_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "role_assignments")]
pub struct RoleAssignment {
    pub id: i32,
    pub assignment_key: String,
    pub school_id: i32,
    pub user_id: i32,
    pub role: String,
    pub granted_by: i32,
    pub valid_from_epoch: i64,
    pub expires_at_epoch: i64,
    pub status: String,
    pub reason: String,
    pub revocation_key: Option<String>,
    pub revoked_by: Option<i32>,
    pub revoked_at_epoch: Option<i64>,
    pub revocation_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for RoleAssignment {
    fn nexus_table() -> &'static str { "role_assignments" }
    fn nexus_label() -> &'static str { "Education Role Assignments" }
    fn nexus_icon() -> &'static str { "🪪" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "assignment_key", label: "Assignment Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "school_id", label: "School", kind: FieldKind::ForeignKey { table: "schools", label_col: "name" }, hidden: false, readonly: true },
            FieldMeta { name: "user_id", label: "Subject", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "role", label: "Role", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "granted_by", label: "Granted By", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "valid_from_epoch", label: "Valid From", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "expires_at_epoch", label: "Expires At", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "reason", label: "Reason", kind: FieldKind::Textarea, hidden: false, readonly: true },
            FieldMeta { name: "revocation_key", label: "Revocation Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "revoked_by", label: "Revoked By", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "revoked_at_epoch", label: "Revoked At", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "revocation_reason", label: "Revocation Reason", kind: FieldKind::Textarea, hidden: false, readonly: true },
        ]
    }
}
"##;

const ROLE_SERVICE: &str = r##"use crate::services::school_service;
use rullst_security::{RbacGuard, UserContext};

pub const EDUCATION_ROLES: [&str; 8] = [
    "school_owner", "admin", "instructor", "assessor", "moderator", "support",
    "guardian", "student",
];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RoleAssignmentReceipt {
    pub assignment_key: String,
    pub user_id: i32,
    pub role: String,
    pub expires_at_epoch: i64,
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RoleRevocationReceipt {
    pub assignment_key: String,
    pub revocation_key: String,
    pub revoked_by: i32,
    pub applied: bool,
}

#[derive(Debug)]
pub enum RoleError {
    Forbidden,
    NotFound,
    InvalidField(&'static str),
    IdempotencyConflict,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for RoleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("education role access denied"),
            Self::NotFound => formatter.write_str("education role assignment not found"),
            Self::InvalidField(field) => write!(formatter, "invalid education role field: {field}"),
            Self::IdempotencyConflict => formatter.write_str("role assignment key is bound to another request"),
            Self::Database(error) => write!(formatter, "education role database error: {error}"),
        }
    }
}

impl std::error::Error for RoleError {}

impl From<rullst_orm::Error> for RoleError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

fn actor_id(context: &UserContext) -> Result<i32, RoleError> {
    context.user_id.parse::<i32>().map_err(|_| RoleError::Forbidden)
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
        })
}

fn valid_reason(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

pub async fn grant_role(
    context: &UserContext,
    assignment_key: &str,
    user_id: i32,
    role: &str,
    valid_from_epoch: i64,
    expires_at_epoch: i64,
    reason: &str,
) -> Result<RoleAssignmentReceipt, RoleError> {
    let actor = actor_id(context)?;
    if user_id <= 0
        || actor <= 0
        || !valid_key(assignment_key, 128)
        || !EDUCATION_ROLES.contains(&role)
        || valid_from_epoch <= 0
        || expires_at_epoch < 0
        || (expires_at_epoch > 0 && expires_at_epoch <= valid_from_epoch)
        || (role == "support" && expires_at_epoch == 0)
        || !valid_reason(reason)
    {
        return Err(RoleError::InvalidField("assignment"));
    }
    let privileged = matches!(role, "school_owner" | "admin");
    if privileged {
        if !context.has_role("school_owner") {
            return Err(RoleError::Forbidden);
        }
        if actor == user_id {
            return Err(RoleError::Forbidden);
        }
    } else if !context.has_role("admin") && !context.has_role("school_owner") {
        return Err(RoleError::Forbidden);
    }
    let school_id = school_service::authorize_school_membership_at(
        context,
        user_id,
        valid_from_epoch,
    ).await.map_err(|error| match error {
        school_service::SchoolError::Database(error) => RoleError::Database(error),
        _ => RoleError::Forbidden,
    })?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool.begin().await.map_err(|error| RoleError::Database(error.into()))?;
    let replay_sql = match driver {
        "postgres" => "SELECT school_id, user_id, role, granted_by, valid_from_epoch, expires_at_epoch, reason FROM role_assignments WHERE assignment_key = $1",
        _ => "SELECT school_id, user_id, role, granted_by, valid_from_epoch, expires_at_epoch, reason FROM role_assignments WHERE assignment_key = ?",
    };
    if let Some(existing) = rullst::db::sqlx::query_as::<_, (i32, i32, String, i32, i64, i64, String)>(replay_sql)
        .bind(assignment_key).fetch_optional(&mut *transaction).await
        .map_err(|error| RoleError::Database(error.into()))?
    {
        if existing != (school_id, user_id, role.to_string(), actor, valid_from_epoch, expires_at_epoch, reason.to_string()) {
            return Err(RoleError::IdempotencyConflict);
        }
        transaction.commit().await.map_err(|error| RoleError::Database(error.into()))?;
        return Ok(RoleAssignmentReceipt { assignment_key: assignment_key.to_string(), user_id, role: role.to_string(), expires_at_epoch, applied: false });
    }
    let insert_sql = match driver {
        "postgres" => "INSERT INTO role_assignments (assignment_key, school_id, user_id, role, granted_by, valid_from_epoch, expires_at_epoch, status, reason, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO role_assignments (assignment_key, school_id, user_id, role, granted_by, valid_from_epoch, expires_at_epoch, status, reason, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(insert_sql).bind(assignment_key).bind(school_id).bind(user_id).bind(role)
        .bind(actor).bind(valid_from_epoch).bind(expires_at_epoch).bind("active").bind(reason)
        .execute(&mut *transaction).await.map_err(|error| RoleError::Database(error.into()))?;
    transaction.commit().await.map_err(|error| RoleError::Database(error.into()))?;
    Ok(RoleAssignmentReceipt { assignment_key: assignment_key.to_string(), user_id, role: role.to_string(), expires_at_epoch, applied: true })
}

pub async fn active_roles_at(
    context: &UserContext,
    user_id: i32,
    observed_at_epoch: i64,
) -> Result<Vec<String>, RoleError> {
    if user_id <= 0 || observed_at_epoch <= 0 { return Err(RoleError::InvalidField("role query")); }
    RbacGuard::authorize_owner_or_role(context, &user_id.to_string(), "admin")
        .map_err(|_| RoleError::Forbidden)?;
    let school_id = school_service::authorize_school_membership_at(
        context,
        user_id,
        observed_at_epoch,
    ).await.map_err(|error| match error {
        school_service::SchoolError::Database(error) => RoleError::Database(error),
        _ => RoleError::Forbidden,
    })?;
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT DISTINCT role FROM role_assignments WHERE school_id = $1 AND user_id = $2 AND status = $3 AND valid_from_epoch <= $4 AND (expires_at_epoch = 0 OR expires_at_epoch > $5) ORDER BY role ASC",
        _ => "SELECT DISTINCT role FROM role_assignments WHERE school_id = ? AND user_id = ? AND status = ? AND valid_from_epoch <= ? AND (expires_at_epoch = 0 OR expires_at_epoch > ?) ORDER BY role ASC",
    };
    let roles = rullst::db::sqlx::query_scalar::<_, String>(sql).bind(school_id).bind(user_id).bind("active")
        .bind(observed_at_epoch).bind(observed_at_epoch)
        .fetch_all(rullst::db::Orm::pool()?).await
        .map_err(|error| RoleError::Database(error.into()))?;
    if roles.len() > EDUCATION_ROLES.len() || roles.iter().any(|role| !EDUCATION_ROLES.contains(&role.as_str())) {
        return Err(RoleError::InvalidField("stored role"));
    }
    Ok(if roles.is_empty() { vec!["student".to_string()] } else { roles })
}

pub async fn revoke_role_at(
    context: &UserContext,
    revocation_key: &str,
    assignment_key: &str,
    observed_at_epoch: i64,
    reason: &str,
) -> Result<RoleRevocationReceipt, RoleError> {
    let actor = actor_id(context)?;
    if actor <= 0
        || !valid_key(revocation_key, 128)
        || !valid_key(assignment_key, 128)
        || observed_at_epoch <= 0
        || !valid_reason(reason)
    {
        return Err(RoleError::InvalidField("revocation"));
    }
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let school_id = school_service::context_school_id(context).await.map_err(|error| match error {
        school_service::SchoolError::Database(error) => RoleError::Database(error),
        _ => RoleError::Forbidden,
    })?;
    let mut transaction = pool.begin().await.map_err(|error| RoleError::Database(error.into()))?;
    let fetch_sql = match driver {
        "postgres" => "SELECT user_id, role, status, revocation_key, revoked_by, revocation_reason FROM role_assignments WHERE assignment_key = $1 AND school_id = $2",
        _ => "SELECT user_id, role, status, revocation_key, revoked_by, revocation_reason FROM role_assignments WHERE assignment_key = ? AND school_id = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (i32, String, String, Option<String>, Option<i32>, Option<String>)>(fetch_sql)
        .bind(assignment_key).bind(school_id).fetch_optional(&mut *transaction).await
        .map_err(|error| RoleError::Database(error.into()))?
        .ok_or(RoleError::NotFound)?;
    let privileged = matches!(row.1.as_str(), "school_owner" | "admin");
    if privileged {
        if !context.has_role("school_owner") || actor == row.0 { return Err(RoleError::Forbidden); }
    } else if !context.has_role("admin") && !context.has_role("school_owner") {
        return Err(RoleError::Forbidden);
    }
    if row.2 == "revoked" {
        if row.3.as_deref() != Some(revocation_key)
            || row.4 != Some(actor)
            || row.5.as_deref() != Some(reason)
        {
            return Err(RoleError::IdempotencyConflict);
        }
        transaction.commit().await.map_err(|error| RoleError::Database(error.into()))?;
        return Ok(RoleRevocationReceipt { assignment_key: assignment_key.to_string(), revocation_key: revocation_key.to_string(), revoked_by: actor, applied: false });
    }
    if row.2 != "active" { return Err(RoleError::IdempotencyConflict); }
    let duplicate_sql = match driver {
        "postgres" => "SELECT assignment_key FROM role_assignments WHERE revocation_key = $1",
        _ => "SELECT assignment_key FROM role_assignments WHERE revocation_key = ?",
    };
    if let Some(existing) = rullst::db::sqlx::query_scalar::<_, String>(duplicate_sql)
        .bind(revocation_key).fetch_optional(&mut *transaction).await
        .map_err(|error| RoleError::Database(error.into()))?
    {
        if existing != assignment_key { return Err(RoleError::IdempotencyConflict); }
    }
    let update_sql = match driver {
        "postgres" => "UPDATE role_assignments SET status = $1, revocation_key = $2, revoked_by = $3, revoked_at_epoch = $4, revocation_reason = $5, updated_at = CURRENT_TIMESTAMP WHERE assignment_key = $6 AND school_id = $7 AND status = $8",
        _ => "UPDATE role_assignments SET status = ?, revocation_key = ?, revoked_by = ?, revoked_at_epoch = ?, revocation_reason = ?, updated_at = CURRENT_TIMESTAMP WHERE assignment_key = ? AND school_id = ? AND status = ?",
    };
    let changed = rullst::db::sqlx::query(update_sql).bind("revoked").bind(revocation_key)
        .bind(actor).bind(observed_at_epoch).bind(reason).bind(assignment_key).bind(school_id).bind("active")
        .execute(&mut *transaction).await.map_err(|error| RoleError::Database(error.into()))?
        .rows_affected() == 1;
    if !changed { return Err(RoleError::IdempotencyConflict); }
    transaction.commit().await.map_err(|error| RoleError::Database(error.into()))?;
    Ok(RoleRevocationReceipt { assignment_key: assignment_key.to_string(), revocation_key: revocation_key.to_string(), revoked_by: actor, applied: true })
}
"##;

const ROLE_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m20260902000000_add_education_roles" }
    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("role_assignments", |table| {
            table.id(); table.string("assignment_key").not_null(); table.integer("school_id").not_null();
            table.integer("user_id").not_null();
            table.string("role").not_null(); table.integer("granted_by").not_null();
            table.big_integer("valid_from_epoch").not_null(); table.big_integer("expires_at_epoch").not_null();
            table.string("status").not_null(); table.string("reason").not_null();
            table.string("revocation_key").nullable(); table.integer("revoked_by").nullable();
            table.big_integer("revoked_at_epoch").nullable(); table.string("revocation_reason").nullable();
            table.timestamps();
        }).await?;
        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX role_assignments_key_unique ON role_assignments(assignment_key)",
            "CREATE UNIQUE INDEX role_assignments_revocation_key_unique ON role_assignments(revocation_key)",
            "CREATE INDEX role_assignments_active_idx ON role_assignments(school_id, user_id, status, valid_from_epoch, expires_at_epoch)",
            "CREATE INDEX role_assignments_grantor_idx ON role_assignments(school_id, granted_by, created_at)",
        ] { sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?; }
        Ok(())
    }
    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("role_assignments").await
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::{ROLE_ASSIGNMENT_MODEL, ROLE_MIGRATION, ROLE_SERVICE};

    #[test]
    fn roles_are_server_scoped_temporary_and_idempotent() {
        assert!(ROLE_ASSIGNMENT_MODEL.contains("readonly: true"));
        assert!(ROLE_MIGRATION.contains("role_assignments_active_idx"));
        assert!(ROLE_SERVICE.contains("role == \"support\" && expires_at_epoch == 0"));
        assert!(ROLE_SERVICE.contains("IdempotencyConflict"));
        assert!(ROLE_SERVICE.contains("actor == user_id"));
        assert!(ROLE_SERVICE.contains("revoke_role_at"));
        assert!(ROLE_MIGRATION.contains("role_assignments_revocation_key_unique"));
    }
}
