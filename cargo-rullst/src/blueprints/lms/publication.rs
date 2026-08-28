// Versioned course-publication and enrollment-pin templates.

#[path = "publication_controller.rs"]
mod publication_controller;
#[path = "publication_rollback.rs"]
mod publication_rollback;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = vec![
        (
            "src/models/course_version.rs",
            COURSE_VERSION_MODEL.to_string(),
        ),
        (
            "src/services/publication_service.rs",
            PUBLICATION_SERVICE.to_string(),
        ),
        (
            "src/migrations/m20260901000000_add_course_publication.rs",
            PUBLICATION_MIGRATION.to_string(),
        ),
    ];
    files.extend(publication_controller::get_files());
    files.extend(publication_rollback::get_files());
    files
}

const COURSE_VERSION_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "course_versions")]
pub struct CourseVersion {
    pub id: i32,
    pub course_id: i32,
    pub version_key: String,
    pub revision: i32,
    pub status: String,
    pub content_json: String,
    pub authored_by: i32,
    pub reviewed_by: i32,
    pub scheduled_at_epoch: i64,
    pub published_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for CourseVersion {
    fn nexus_table() -> &'static str { "course_versions" }
    fn nexus_label() -> &'static str { "Course Versions" }
    fn nexus_icon() -> &'static str { "📚" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "version_key", label: "Version Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "revision", label: "Revision", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "status", label: "State", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "content_json", label: "Immutable Snapshot", kind: FieldKind::Json, hidden: false, readonly: true },
            FieldMeta { name: "authored_by", label: "Author", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "reviewed_by", label: "Reviewer", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "scheduled_at_epoch", label: "Scheduled Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "published_at_epoch", label: "Published Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
        ]
    }
}
"##;

const PUBLICATION_SERVICE: &str = r##"use crate::models::course::Course;
use crate::services::school_service;
use rullst_security::{RbacGuard, UserContext};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CourseVersionReceipt {
    pub id: i32,
    pub course_id: i32,
    pub version_key: String,
    pub revision: i32,
    pub status: String,
    pub applied: bool,
}

#[derive(Debug)]
pub enum PublicationError {
    Forbidden,
    NotFound,
    InvalidField(&'static str),
    InvalidState,
    SeparationOfDuties,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for PublicationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("course publication access denied"),
            Self::NotFound => formatter.write_str("course version not found"),
            Self::InvalidField(field) => write!(formatter, "invalid course publication field: {field}"),
            Self::InvalidState => formatter.write_str("course version state transition rejected"),
            Self::SeparationOfDuties => formatter.write_str("course author cannot approve the same version"),
            Self::Database(error) => write!(formatter, "course publication database error: {error}"),
        }
    }
}

impl std::error::Error for PublicationError {}

impl From<rullst_orm::Error> for PublicationError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

fn actor_id(context: &UserContext) -> Result<i32, PublicationError> {
    context.user_id.parse::<i32>().map_err(|_| PublicationError::Forbidden)
}

fn authorize_author(context: &UserContext) -> Result<i32, PublicationError> {
    if RbacGuard::authorize(context, "instructor").is_err()
        && RbacGuard::authorize(context, "admin").is_err()
    {
        return Err(PublicationError::Forbidden);
    }
    actor_id(context)
}

fn authorize_reviewer(context: &UserContext) -> Result<i32, PublicationError> {
    RbacGuard::authorize(context, "admin").map_err(|_| PublicationError::Forbidden)?;
    actor_id(context)
}

async fn authorize_course_scope(
    context: &UserContext,
    course_id: i32,
) -> Result<(), PublicationError> {
    school_service::authorize_course(context, course_id).await
        .map(|_| ())
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => PublicationError::Database(error),
            _ => PublicationError::Forbidden,
        })
}

async fn authorize_version_scope(
    context: &UserContext,
    version_id: i32,
) -> Result<(), PublicationError> {
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT course_id FROM course_versions WHERE id = $1",
        _ => "SELECT course_id FROM course_versions WHERE id = ?",
    };
    let course_id = rullst::db::sqlx::query_scalar::<_, i32>(sql).bind(version_id)
        .fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| PublicationError::Database(error.into()))?
        .ok_or(PublicationError::NotFound)?;
    authorize_course_scope(context, course_id).await
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
        })
}

pub async fn create_draft(
    context: &UserContext,
    course_id: i32,
    version_key: &str,
    content_json: &str,
) -> Result<CourseVersionReceipt, PublicationError> {
    let author = authorize_author(context)?;
    if course_id <= 0
        || !valid_key(version_key, 96)
        || content_json.is_empty()
        || content_json.len() > 1_048_576
        || !serde_json::from_str::<serde_json::Value>(content_json)
            .is_ok_and(|value| value.is_object())
    {
        return Err(PublicationError::InvalidField("draft"));
    }
    authorize_course_scope(context, course_id).await?;
    if Course::find(course_id).await?.is_none() {
        return Err(PublicationError::NotFound);
    }
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool.begin().await.map_err(|error| PublicationError::Database(error.into()))?;
    let revision_sql = match driver {
        "postgres" => "SELECT COALESCE(MAX(revision), 0) FROM course_versions WHERE course_id = $1",
        _ => "SELECT COALESCE(MAX(revision), 0) FROM course_versions WHERE course_id = ?",
    };
    let prior = rullst::db::sqlx::query_scalar::<_, i32>(revision_sql)
        .bind(course_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| PublicationError::Database(error.into()))?;
    let revision = prior.checked_add(1).ok_or(PublicationError::InvalidField("revision"))?;
    let insert_sql = match driver {
        "postgres" => "INSERT INTO course_versions (course_id, version_key, revision, status, content_json, authored_by, reviewed_by, scheduled_at_epoch, published_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, 0, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO course_versions (course_id, version_key, revision, status, content_json, authored_by, reviewed_by, scheduled_at_epoch, published_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(insert_sql)
        .bind(course_id).bind(version_key).bind(revision).bind("draft")
        .bind(content_json).bind(author)
        .execute(&mut *transaction).await
        .map_err(|error| PublicationError::Database(error.into()))?;
    let id_sql = match driver {
        "postgres" => "SELECT id FROM course_versions WHERE version_key = $1",
        _ => "SELECT id FROM course_versions WHERE version_key = ?",
    };
    let id = rullst::db::sqlx::query_scalar::<_, i32>(id_sql).bind(version_key)
        .fetch_one(&mut *transaction).await
        .map_err(|error| PublicationError::Database(error.into()))?;
    transaction.commit().await.map_err(|error| PublicationError::Database(error.into()))?;
    Ok(CourseVersionReceipt { id, course_id, version_key: version_key.to_string(), revision, status: "draft".to_string(), applied: true })
}

pub async fn submit_for_review(
    context: &UserContext,
    version_id: i32,
) -> Result<bool, PublicationError> {
    let actor = authorize_author(context)?;
    if version_id <= 0 { return Err(PublicationError::InvalidField("version")); }
    authorize_version_scope(context, version_id).await?;
    let driver = rullst::db::Orm::driver()?;
    let owner_sql = match driver {
        "postgres" => "SELECT authored_by FROM course_versions WHERE id = $1",
        _ => "SELECT authored_by FROM course_versions WHERE id = ?",
    };
    let owner = rullst::db::sqlx::query_scalar::<_, i32>(owner_sql).bind(version_id)
        .fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| PublicationError::Database(error.into()))?
        .ok_or(PublicationError::NotFound)?;
    RbacGuard::authorize_owner_or_role(context, &owner.to_string(), "admin")
        .map_err(|_| PublicationError::Forbidden)?;
    let update_sql = match driver {
        "postgres" => "UPDATE course_versions SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2 AND authored_by = $3 AND status = $4",
        _ => "UPDATE course_versions SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND authored_by = ? AND status = ?",
    };
    let changed = rullst::db::sqlx::query(update_sql).bind("review").bind(version_id)
        .bind(owner).bind("draft").execute(rullst::db::Orm::pool()?).await
        .map_err(|error| PublicationError::Database(error.into()))?.rows_affected() == 1;
    if !changed && actor != owner { return Err(PublicationError::Forbidden); }
    Ok(changed)
}

pub async fn review_version_at(
    context: &UserContext,
    version_id: i32,
    publish_at_epoch: i64,
    now_epoch: i64,
) -> Result<CourseVersionReceipt, PublicationError> {
    let reviewer = authorize_reviewer(context)?;
    if version_id <= 0 || publish_at_epoch < 0 || now_epoch <= 0 {
        return Err(PublicationError::InvalidField("review"));
    }
    authorize_version_scope(context, version_id).await?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool.begin().await.map_err(|error| PublicationError::Database(error.into()))?;
    let fetch_sql = match driver {
        "postgres" => "SELECT course_id, version_key, revision, status, authored_by, scheduled_at_epoch, reviewed_by FROM course_versions WHERE id = $1",
        _ => "SELECT course_id, version_key, revision, status, authored_by, scheduled_at_epoch, reviewed_by FROM course_versions WHERE id = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (i32, String, i32, String, i32, i64, i32)>(fetch_sql)
        .bind(version_id).fetch_optional(&mut *transaction).await
        .map_err(|error| PublicationError::Database(error.into()))?
        .ok_or(PublicationError::NotFound)?;
    let activating_scheduled = row.3 == "scheduled" && row.5 > 0 && row.5 <= now_epoch;
    if (row.3 == "review" && row.4 == reviewer)
        || (activating_scheduled && (row.6 <= 0 || row.6 == row.4))
    {
        return Err(PublicationError::SeparationOfDuties);
    }
    if row.3 != "review" && !activating_scheduled { return Err(PublicationError::InvalidState); }
    if row.3 == "review" && publish_at_epoch > now_epoch {
        let schedule_sql = match driver {
            "postgres" => "UPDATE course_versions SET status = $1, reviewed_by = $2, scheduled_at_epoch = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $4 AND status = $5",
            _ => "UPDATE course_versions SET status = ?, reviewed_by = ?, scheduled_at_epoch = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = ?",
        };
        let changed = rullst::db::sqlx::query(schedule_sql).bind("scheduled").bind(reviewer)
            .bind(publish_at_epoch).bind(version_id).bind("review")
            .execute(&mut *transaction).await
            .map_err(|error| PublicationError::Database(error.into()))?.rows_affected() == 1;
        if !changed { return Err(PublicationError::InvalidState); }
        transaction.commit().await.map_err(|error| PublicationError::Database(error.into()))?;
        return Ok(CourseVersionReceipt { id: version_id, course_id: row.0, version_key: row.1, revision: row.2, status: "scheduled".to_string(), applied: true });
    }
    let archive_sql = match driver {
        "postgres" => "UPDATE course_versions SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE course_id = $2 AND status = $3 AND id <> $4",
        _ => "UPDATE course_versions SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE course_id = ? AND status = ? AND id <> ?",
    };
    rullst::db::sqlx::query(archive_sql).bind("archived").bind(row.0).bind("published")
        .bind(version_id).execute(&mut *transaction).await
        .map_err(|error| PublicationError::Database(error.into()))?;
    let publish_sql = match driver {
        "postgres" => "UPDATE course_versions SET status = $1, reviewed_by = $2, published_at_epoch = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $4 AND status IN ($5, $6)",
        _ => "UPDATE course_versions SET status = ?, reviewed_by = ?, published_at_epoch = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status IN (?, ?)",
    };
    let recorded_reviewer = if activating_scheduled { row.6 } else { reviewer };
    let changed = rullst::db::sqlx::query(publish_sql).bind("published").bind(recorded_reviewer)
        .bind(now_epoch).bind(version_id).bind("review").bind("scheduled")
        .execute(&mut *transaction).await
        .map_err(|error| PublicationError::Database(error.into()))?.rows_affected() == 1;
    if !changed { return Err(PublicationError::InvalidState); }
    let event_key = format!("course-published:{}", row.1);
    let payload = serde_json::json!({"schema_version":1,"actor_user_id":reviewer,"course_id":row.0,"course_version_id":version_id,"version_key":row.1,"revision":row.2}).to_string();
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => PublicationError::Database(error),
            _ => PublicationError::Forbidden,
        })?;
    let event_sql = match driver {
        "postgres" => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, 0, $4, $5, 0, $6, $7, $8, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, 0, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, 0, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    rullst::db::sqlx::query(event_sql).bind(school_id).bind(event_key).bind("course_published").bind(payload)
        .bind("pending").bind("").bind("").bind("").execute(&mut *transaction).await
        .map_err(|error| PublicationError::Database(error.into()))?;
    transaction.commit().await.map_err(|error| PublicationError::Database(error.into()))?;
    Ok(CourseVersionReceipt { id: version_id, course_id: row.0, version_key: row.1, revision: row.2, status: "published".to_string(), applied: true })
}
"##;

const PUBLICATION_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m20260901000000_add_course_publication" }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("course_versions", |table| {
            table.id(); table.integer("course_id").not_null(); table.string("version_key").not_null();
            table.integer("revision").not_null(); table.string("status").not_null();
            table.string("content_json").not_null(); table.integer("authored_by").not_null();
            table.integer("reviewed_by").not_null(); table.big_integer("scheduled_at_epoch").not_null();
            table.big_integer("published_at_epoch").not_null(); table.timestamps();
        }).await?;
        Schema::create("enrollment_content_versions", |table| {
            table.id(); table.integer("enrollment_id").not_null();
            table.integer("course_version_id").not_null(); table.timestamps();
        }).await?;
        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX course_versions_key_unique ON course_versions(version_key)",
            "CREATE UNIQUE INDEX course_versions_revision_unique ON course_versions(course_id, revision)",
            "CREATE INDEX course_versions_status_idx ON course_versions(course_id, status, scheduled_at_epoch)",
            "CREATE UNIQUE INDEX enrollment_content_version_unique ON enrollment_content_versions(enrollment_id)",
        ] { sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?; }
        for fixture in [
            "INSERT INTO course_versions (id, course_id, version_key, revision, status, content_json, authored_by, reviewed_by, scheduled_at_epoch, published_at_epoch, created_at, updated_at) VALUES (1, 1, 'course-1-v1', 1, 'published', '{\"schema_version\":1,\"lesson_ids\":[1,2],\"completion\":{\"schema_version\":1,\"ruleset_version\":\"course-1-completion-v1\",\"required_lesson_ids\":[1,2],\"required_progress_percent\":100}}', 1, 2, 0, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            "INSERT INTO course_versions (id, course_id, version_key, revision, status, content_json, authored_by, reviewed_by, scheduled_at_epoch, published_at_epoch, created_at, updated_at) VALUES (2, 2, 'course-2-v1', 1, 'published', '{\"schema_version\":1,\"lesson_ids\":[3,4],\"completion\":{\"schema_version\":1,\"ruleset_version\":\"course-2-completion-v1\",\"required_lesson_ids\":[3,4],\"required_progress_percent\":100}}', 1, 2, 0, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        ] { sqlx::query(sqlx::AssertSqlSafe(fixture)).execute(pool).await?; }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("enrollment_content_versions").await?;
        Schema::drop_if_exists("course_versions").await
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::{COURSE_VERSION_MODEL, PUBLICATION_MIGRATION, PUBLICATION_SERVICE};

    #[test]
    fn publication_is_versioned_reviewed_pinned_and_transactional() {
        assert!(COURSE_VERSION_MODEL.contains("Immutable Snapshot"));
        assert!(PUBLICATION_MIGRATION.contains("enrollment_content_version_unique"));
        assert!(PUBLICATION_SERVICE.contains("SeparationOfDuties"));
        assert!(PUBLICATION_SERVICE.contains("recorded_reviewer"));
        assert!(PUBLICATION_SERVICE.contains("course_published"));
        assert!(!PUBLICATION_SERVICE.contains("format!(\"SELECT"));
    }
}
