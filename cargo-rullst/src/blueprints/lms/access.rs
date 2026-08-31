// Authenticated enrollment, entitlement and progress templates for the LMS starter.

mod auth_middleware;

use auth_middleware::AUTH_MIDDLEWARE;

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/services/learning_service.rs",
            LEARNING_SERVICE.to_string(),
        ),
        (
            "src/services/mod.rs",
            "pub mod activity_contract;\npub mod assessment_service;\npub mod assessment_timing_service;\npub mod assignment_grade_correction_service;\npub mod assignment_grading_service;\npub mod assignment_submission_service;\npub mod automation_execution_service;\npub mod automation_service;\npub mod automation_worker_event_service;\npub mod automation_worker_service;\npub mod completion_service;\npub mod learning_service;\npub mod notification_service;\npub mod notification_template_service;\npub mod outbox_service;\npub mod privacy_request_executor_service;\npub mod privacy_request_worker_service;\npub mod privacy_retention_service;\npub mod privacy_service;\npub mod progress_service;\npub mod publication_rollback_service;\npub mod publication_scheduler_service;\npub mod publication_service;\npub mod role_service;\npub mod scheduler_lease_service;\npub mod school_service;\npub mod score_correction_service;\npub mod score_service;\n"
                .to_string(),
        ),
        (
            "src/controllers/learning_controller.rs",
            LEARNING_CONTROLLER.to_string(),
        ),
        (
            "src/middlewares/auth_middleware.rs",
            AUTH_MIDDLEWARE.to_string(),
        ),
        (
            "src/middlewares/mod.rs",
            "pub mod auth_middleware;\n".to_string(),
        ),
    ]
}

const LEARNING_SERVICE: &str = r##"use crate::models::course::Course;
use crate::models::enrollment::Enrollment;
use crate::models::lesson::Lesson;
use crate::models::lesson_progress::LessonProgress;
use crate::services::school_service::{self, SchoolError};
use rullst_security::{RbacGuard, UserContext};

#[derive(Debug)]
pub enum LearningError {
    NotFound(&'static str),
    Forbidden,
    NotReleased,
    Expired,
    PrerequisiteNotMet,
    InvalidAvailabilityPolicy,
    InvalidContentVersion,
    InvalidProgress,
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for LearningError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(resource) => write!(formatter, "{resource} not found"),
            Self::Forbidden => formatter.write_str("learning resource access denied"),
            Self::NotReleased => formatter.write_str("lesson is not released"),
            Self::Expired => formatter.write_str("lesson access has expired"),
            Self::PrerequisiteNotMet => formatter.write_str("lesson prerequisite is not met"),
            Self::InvalidAvailabilityPolicy => formatter.write_str("lesson availability policy is invalid"),
            Self::InvalidContentVersion => formatter.write_str("course has no unambiguous published content version"),
            Self::InvalidProgress => formatter.write_str("progress must be between 0 and 100"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "learning database error: {error}"),
        }
    }
}

impl std::error::Error for LearningError {}

impl From<rullst_orm::Error> for LearningError {
    fn from(error: rullst_orm::Error) -> Self {
        Self::Database(error)
    }
}

fn authorize_identity(context: &UserContext, user_id: i32) -> Result<(), LearningError> {
    RbacGuard::authorize_owner_or_role(context, &user_id.to_string(), "admin")
        .map_err(|_| LearningError::Forbidden)
}

fn map_school_error(error: SchoolError) -> LearningError {
    match error {
        SchoolError::Database(error) => LearningError::Database(error),
        SchoolError::Forbidden
        | SchoolError::AmbiguousMembership
        | SchoolError::InvalidField(_) => LearningError::Forbidden,
    }
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
        })
}

fn unix_now() -> Result<i64, LearningError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| LearningError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| LearningError::Clock)
}

pub async fn enroll(
    user_id: i32,
    context: &UserContext,
    course_id: i32,
) -> Result<Enrollment, LearningError> {
    authorize_identity(context, user_id)?;
    if user_id <= 0 || course_id <= 0 {
        return Err(LearningError::Forbidden);
    }
    let school_id = school_service::authorize_course_enrollment_at(context, user_id, course_id, unix_now()?)
        .await
        .map_err(map_school_error)?;
    if Course::find(course_id).await?.is_none() {
        return Err(LearningError::NotFound("course"));
    }
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| LearningError::Database(error.into()))?;
    let upsert_sql = match driver {
        "postgres" => "INSERT INTO enrollments (user_id, course_id, status, created_at, updated_at) VALUES ($1, $2, $3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id) DO UPDATE SET status = EXCLUDED.status, updated_at = CURRENT_TIMESTAMP",
        "mysql" => "INSERT INTO enrollments (user_id, course_id, status, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE status = VALUES(status), updated_at = CURRENT_TIMESTAMP",
        _ => "INSERT INTO enrollments (user_id, course_id, status, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id) DO UPDATE SET status = excluded.status, updated_at = CURRENT_TIMESTAMP",
    };
    rullst::db::sqlx::query(upsert_sql)
        .bind(user_id)
        .bind(course_id)
        .bind("active")
        .execute(&mut *transaction)
        .await
        .map_err(|error| LearningError::Database(error.into()))?;
    let enrollment_sql = match driver {
        "postgres" => "SELECT id, user_id, course_id, status, created_at, updated_at FROM enrollments WHERE user_id = $1 AND course_id = $2 AND status = $3",
        _ => "SELECT id, user_id, course_id, status, created_at, updated_at FROM enrollments WHERE user_id = ? AND course_id = ? AND status = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (i32, i32, i32, String, String, String)>(
        enrollment_sql,
    )
    .bind(user_id)
    .bind(course_id)
    .bind("active")
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| LearningError::Database(error.into()))?;
    let version_sql = match driver {
        "postgres" => "SELECT id FROM course_versions WHERE course_id = $1 AND status = $2 ORDER BY revision DESC",
        _ => "SELECT id FROM course_versions WHERE course_id = ? AND status = ? ORDER BY revision DESC",
    };
    let mut versions = rullst::db::sqlx::query_scalar::<_, i32>(version_sql)
        .bind(course_id)
        .bind("published")
        .fetch_all(&mut *transaction)
        .await
        .map_err(|error| LearningError::Database(error.into()))?;
    if versions.len() != 1 {
        return Err(LearningError::InvalidContentVersion);
    }
    let version_id = versions
        .pop()
        .ok_or(LearningError::InvalidContentVersion)?;
    let pin_sql = match driver {
        "postgres" => "INSERT INTO enrollment_content_versions (enrollment_id, course_version_id, created_at, updated_at) VALUES ($1, $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO enrollment_content_versions (enrollment_id, course_version_id, created_at, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO enrollment_content_versions (enrollment_id, course_version_id, created_at, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    rullst::db::sqlx::query(pin_sql)
        .bind(row.0)
        .bind(version_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| LearningError::Database(error.into()))?;
    let event_key = format!("enrollment:{user_id}:{course_id}");
    let payload_json = serde_json::json!({
        "schema_version": 1,
        "actor_user_id": user_id,
        "subject_user_id": user_id,
        "course_id": course_id,
        "enrollment_id": row.0,
        "status": "active",
    })
    .to_string();
    let event_sql = match driver {
        "postgres" => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, $11, $12, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    rullst::db::sqlx::query(event_sql)
        .bind(school_id)
        .bind(event_key)
        .bind("enrollment_activated")
        .bind(user_id)
        .bind(payload_json)
        .bind("pending")
        .bind(0_i32)
        .bind("")
        .bind("")
        .bind("")
        .bind(0_i64)
        .bind(0_i64)
        .execute(&mut *transaction)
        .await
        .map_err(|error| LearningError::Database(error.into()))?;
    transaction
        .commit()
        .await
        .map_err(|error| LearningError::Database(error.into()))?;
    Ok(Enrollment {
        id: row.0,
        user_id: row.1,
        course_id: row.2,
        status: row.3,
        created_at: row.4,
        updated_at: row.5,
    })
}

pub async fn authorize_lesson(
    user_id: i32,
    context: &UserContext,
    lesson_id: i32,
) -> Result<Lesson, LearningError> {
    authorize_lesson_at(user_id, context, lesson_id, unix_now()?).await
}

pub async fn authorize_lesson_at(
    user_id: i32,
    context: &UserContext,
    lesson_id: i32,
    observed_at_epoch: i64,
) -> Result<Lesson, LearningError> {
    if observed_at_epoch <= 0 {
        return Err(LearningError::Clock);
    }
    authorize_identity(context, user_id)?;
    let scoped_course_id = school_service::authorize_lesson(context, lesson_id)
        .await
        .map_err(map_school_error)?;
    let lesson = Lesson::find(lesson_id)
        .await?
        .ok_or(LearningError::NotFound("lesson"))?;
    if lesson.course_id != scoped_course_id {
        return Err(LearningError::Forbidden);
    }
    school_service::authorize_course_enrollment_at(
        context,
        user_id,
        lesson.course_id,
        observed_at_epoch,
    )
    .await
    .map_err(map_school_error)?;
    let enrollment = Enrollment::active_for(user_id, lesson.course_id)
        .await?
        .ok_or(LearningError::Forbidden)?;
    RbacGuard::authorize_owner_or_role(context, &enrollment.user_id.to_string(), "admin")
        .map_err(|_| LearningError::Forbidden)?;

    let driver = rullst::db::Orm::driver()?;
    let policy_sql = match driver {
        "postgres" => "SELECT ruleset_version, release_at_epoch, expire_at_epoch, prerequisite_lesson_id, required_progress_percent FROM lesson_release_rules WHERE lesson_id = $1 AND status = $2 ORDER BY id ASC",
        _ => "SELECT ruleset_version, release_at_epoch, expire_at_epoch, prerequisite_lesson_id, required_progress_percent FROM lesson_release_rules WHERE lesson_id = ? AND status = ? ORDER BY id ASC",
    };
    let mut policies = rullst::db::sqlx::query_as::<_, (String, i64, i64, i32, i32)>(policy_sql)
        .bind(lesson_id)
        .bind("active")
        .fetch_all(rullst::db::Orm::pool()?)
        .await
        .map_err(|error| LearningError::Database(error.into()))?;
    if policies.len() != 1 {
        return Err(LearningError::InvalidAvailabilityPolicy);
    }
    let policy = policies
        .pop()
        .ok_or(LearningError::InvalidAvailabilityPolicy)?;
    let has_prerequisite = policy.3 > 0;
    if !valid_key(&policy.0, 64)
        || policy.1 < 0
        || policy.2 < 0
        || (policy.1 > 0 && policy.2 > 0 && policy.2 <= policy.1)
        || policy.3 < 0
        || policy.3 == lesson_id
        || policy.4 < 0
        || policy.4 > 100
        || (has_prerequisite && policy.4 == 0)
        || (!has_prerequisite && policy.4 != 0)
    {
        return Err(LearningError::InvalidAvailabilityPolicy);
    }
    if policy.1 > 0 && observed_at_epoch < policy.1 {
        return Err(LearningError::NotReleased);
    }
    if policy.2 > 0 && observed_at_epoch > policy.2 {
        return Err(LearningError::Expired);
    }
    if has_prerequisite {
        let prerequisite = Lesson::find(policy.3)
            .await?
            .ok_or(LearningError::InvalidAvailabilityPolicy)?;
        if prerequisite.course_id != lesson.course_id {
            return Err(LearningError::InvalidAvailabilityPolicy);
        }
        let progress = LessonProgress::for_learner(user_id, policy.3).await?;
        if progress.map_or(0, |value| value.progress_percent) < policy.4 {
            return Err(LearningError::PrerequisiteNotMet);
        }
    }
    Ok(lesson)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_boundary_denies_cross_user_context() {
        let owner = UserContext::new("7", vec!["student".to_string()]);
        let unrelated = UserContext::new("8", vec!["student".to_string()]);

        assert!(authorize_identity(&owner, 7).is_ok());
        assert!(matches!(
            authorize_identity(&unrelated, 7),
            Err(LearningError::Forbidden)
        ));
    }
}
"##;

const LEARNING_CONTROLLER: &str = r##"use crate::models::lesson_progress::LessonProgress;
use crate::pages::lms;
use crate::services::learning_service::{self, LearningError};
use crate::services::progress_service::{self, ProgressError};
use rullst::server::{Extension, Form, IntoResponse, Path, Redirect, Response, StatusCode};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProgressDto {
    pub progress_percent: i32,
    pub idempotency_key: String,
}

fn learning_error_response(error: LearningError) -> Response {
    let status = match &error {
        LearningError::NotFound(_) => StatusCode::NOT_FOUND,
        LearningError::Forbidden => StatusCode::FORBIDDEN,
        LearningError::NotReleased | LearningError::PrerequisiteNotMet => StatusCode::FORBIDDEN,
        LearningError::Expired => StatusCode::GONE,
        LearningError::InvalidAvailabilityPolicy => StatusCode::SERVICE_UNAVAILABLE,
        LearningError::InvalidContentVersion => StatusCode::SERVICE_UNAVAILABLE,
        LearningError::InvalidProgress => StatusCode::UNPROCESSABLE_ENTITY,
        LearningError::Clock => StatusCode::INTERNAL_SERVER_ERROR,
        LearningError::Database(_) => {
            eprintln!("Learning operation failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE
        }
    };
    (status, status.canonical_reason().unwrap_or("Learning request failed")).into_response()
}

fn progress_error_response(error: ProgressError) -> Response {
    match error {
        ProgressError::Access(error) => learning_error_response(error),
        ProgressError::InvalidField(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        ProgressError::Database(error) => {
            eprintln!("Progress operation failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

pub async fn enroll(
    Path(course_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
) -> Response {
    match learning_service::enroll(user_id, &context, course_id).await {
        Ok(_) => Redirect::to(&format!("/courses/{course_id}")).into_response(),
        Err(error) => learning_error_response(error),
    }
}

pub async fn play_lesson(
    Path(lesson_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    Extension(csrf): Extension<rullst::security::CsrfToken>,
    Extension(csp_nonce): Extension<rullst::security::CspNonce>,
) -> Response {
    let lesson = match learning_service::authorize_lesson(user_id, &context, lesson_id).await {
        Ok(lesson) => lesson,
        Err(error) => return learning_error_response(error),
    };
    let progress = match LessonProgress::for_learner(user_id, lesson_id).await {
        Ok(progress) => progress.map_or(0, |value| value.progress_percent),
        Err(error) => return learning_error_response(LearningError::Database(error)),
    };
    let progress_key = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(elapsed) => format!("progress-{}", elapsed.as_nanos()),
        Err(error) => {
            eprintln!("Progress clock unavailable: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    match lms::lesson_player_page(
        &lesson.title,
        &lesson.media_kind,
        &lesson.media_url,
        &lesson.captions_url,
        &lesson.transcript,
        &lesson.language_tag,
        lesson.course_id,
        lesson.id,
        progress,
        csrf.as_str(),
        &progress_key,
        csp_nonce.as_str(),
    ) {
        Ok(page) => rullst::response::Html(page).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub async fn record_progress(
    Path(lesson_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    Form(payload): Form<ProgressDto>,
) -> Response {
    match progress_service::record_progress(
        &context,
        user_id,
        lesson_id,
        payload.progress_percent,
        &payload.idempotency_key,
    )
    .await
    {
        Ok(_) => Redirect::to(&format!("/lessons/{lesson_id}/play")).into_response(),
        Err(error) => progress_error_response(error),
    }
}
"##;
