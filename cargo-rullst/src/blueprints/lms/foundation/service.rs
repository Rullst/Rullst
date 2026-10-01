pub(super) const FOUNDATION_SERVICE: &str = r##"use crate::models::course::Course;
use crate::models::enrollment::Enrollment;
use crate::models::lesson::Lesson;
use crate::models::lesson_progress::LessonProgress;
use rullst_security::{RbacGuard, UserContext};

#[derive(Debug)]
pub enum LearningError {
    NotFound(&'static str),
    Forbidden,
    InvalidField(&'static str),
    IdempotencyConflict,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for LearningError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(resource) => write!(formatter, "{resource} not found"),
            Self::Forbidden => formatter.write_str("learning access denied"),
            Self::InvalidField(field) => write!(formatter, "invalid learning field: {field}"),
            Self::IdempotencyConflict => formatter.write_str("learning idempotency conflict"),
            Self::Database(error) => write!(formatter, "learning database error: {error}"),
        }
    }
}

impl std::error::Error for LearningError {}

impl From<rullst_orm::Error> for LearningError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

#[derive(Debug, Clone)]
pub struct ProgressReceipt {
    pub applied: bool,
    pub progress: LessonProgress,
}

fn authorize_identity(context: &UserContext, user_id: i32) -> Result<(), LearningError> {
    RbacGuard::authorize_owner_or_role(context, &user_id.to_string(), "admin")
        .map_err(|_| LearningError::Forbidden)
}

fn valid_key(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
    })
}

pub async fn enroll(
    context: &UserContext,
    user_id: i32,
    course_id: i32,
) -> Result<Enrollment, LearningError> {
    authorize_identity(context, user_id)?;
    if user_id <= 0 || course_id <= 0 { return Err(LearningError::InvalidField("enrollment")); }
    if Course::find(course_id).await?.is_none() { return Err(LearningError::NotFound("course")); }
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "INSERT INTO enrollments (user_id, course_id, status, created_at, updated_at) VALUES ($1, $2, $3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id) DO UPDATE SET status = EXCLUDED.status, updated_at = CURRENT_TIMESTAMP",
        "mysql" => "INSERT INTO enrollments (user_id, course_id, status, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE status = VALUES(status), updated_at = CURRENT_TIMESTAMP",
        _ => "INSERT INTO enrollments (user_id, course_id, status, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id) DO UPDATE SET status = excluded.status, updated_at = CURRENT_TIMESTAMP",
    };
    rullst::db::sqlx::query(sql).bind(user_id).bind(course_id).bind("active")
        .execute(rullst::db::Orm::pool()?).await
        .map_err(|error| LearningError::Database(error.into()))?;
    Enrollment::active_for(user_id, course_id).await?
        .ok_or(LearningError::NotFound("enrollment"))
}

pub async fn authorize_lesson(
    context: &UserContext,
    user_id: i32,
    lesson_id: i32,
) -> Result<Lesson, LearningError> {
    authorize_identity(context, user_id)?;
    if user_id <= 0 || lesson_id <= 0 { return Err(LearningError::InvalidField("lesson")); }
    let lesson = Lesson::find(lesson_id).await?.ok_or(LearningError::NotFound("lesson"))?;
    if Enrollment::active_for(user_id, lesson.course_id).await?.is_none() {
        return Err(LearningError::Forbidden);
    }
    Ok(lesson)
}

/// The stored receipt when this learner already recorded `idempotency_key`.
async fn replay(
    driver: &str,
    user_id: i32,
    lesson_id: i32,
    progress_percent: i32,
    idempotency_key: &str,
) -> Result<Option<ProgressReceipt>, LearningError> {
    let pool = rullst::db::Orm::pool()?;
    // Keys are unique per learner, so another learner's submission can never
    // claim, replay or block this learner's key.
    let replay_sql = match driver {
        "postgres" => "SELECT subject_user_id, lesson_id, current_percent FROM lesson_progress_events WHERE subject_user_id = $1 AND event_key = $2",
        _ => "SELECT subject_user_id, lesson_id, current_percent FROM lesson_progress_events WHERE subject_user_id = ? AND event_key = ?",
    };
    let Some(replay) = rullst::db::sqlx::query_as::<_, (i32, i32, i32)>(replay_sql)
        .bind(user_id).bind(idempotency_key).fetch_optional(pool).await
        .map_err(|error| LearningError::Database(error.into()))?
    else {
        return Ok(None);
    };
    if replay != (user_id, lesson_id, progress_percent) {
        return Err(LearningError::IdempotencyConflict);
    }
    let progress = LessonProgress::for_learner(user_id, lesson_id).await?
        .ok_or(LearningError::NotFound("progress"))?;
    Ok(Some(ProgressReceipt { applied: false, progress }))
}

pub async fn record_progress(
    context: &UserContext,
    user_id: i32,
    lesson_id: i32,
    progress_percent: i32,
    idempotency_key: &str,
) -> Result<ProgressReceipt, LearningError> {
    if !(0..=100).contains(&progress_percent) || !valid_key(idempotency_key) {
        return Err(LearningError::InvalidField("progress"));
    }
    authorize_lesson(context, user_id, lesson_id).await?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    if let Some(receipt) = replay(driver, user_id, lesson_id, progress_percent, idempotency_key).await? {
        return Ok(receipt);
    }

    let mut transaction = pool.begin().await
        .map_err(|error| LearningError::Database(error.into()))?;
    // The event insert claims the key as the transaction's first write, so a
    // concurrent submission of the same key (a double click) waits for this
    // one and then replays it instead of failing after its own read.
    let event_sql = match driver {
        "postgres" => "INSERT INTO lesson_progress_events (event_key, actor_user_id, subject_user_id, lesson_id, previous_percent, current_percent, event_kind, reason, created_at, updated_at) VALUES ($1, $2, $3, $4, COALESCE((SELECT progress_percent FROM lesson_progress WHERE user_id = $5 AND lesson_id = $6), 0), $7, $8, $9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO lesson_progress_events (event_key, actor_user_id, subject_user_id, lesson_id, previous_percent, current_percent, event_kind, reason, created_at, updated_at) VALUES (?, ?, ?, ?, COALESCE((SELECT progress_percent FROM lesson_progress WHERE user_id = ? AND lesson_id = ?), 0), ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    let claimed = rullst::db::sqlx::query(event_sql).bind(idempotency_key).bind(user_id).bind(user_id)
        .bind(lesson_id).bind(user_id).bind(lesson_id).bind(progress_percent).bind("progress_recorded")
        .bind("").execute(&mut *transaction).await;
    match claimed {
        Ok(_) => {}
        Err(rullst::db::sqlx::Error::Database(error)) if error.is_unique_violation() => {
            transaction.rollback().await.map_err(|error| LearningError::Database(error.into()))?;
            return match replay(driver, user_id, lesson_id, progress_percent, idempotency_key).await? {
                Some(receipt) => Ok(receipt),
                None => Err(LearningError::Database(rullst::db::sqlx::Error::Database(error).into())),
            };
        }
        Err(error) => return Err(LearningError::Database(error.into())),
    }
    // The conflict branches keep the higher stored value, so a new row simply
    // takes the requested percentage.
    let upsert_sql = match driver {
        "postgres" => "INSERT INTO lesson_progress (user_id, lesson_id, progress_percent, completed, created_at, updated_at) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, lesson_id) DO UPDATE SET progress_percent = GREATEST(lesson_progress.progress_percent, EXCLUDED.progress_percent), completed = GREATEST(lesson_progress.completed, EXCLUDED.completed), updated_at = CURRENT_TIMESTAMP",
        "mysql" => "INSERT INTO lesson_progress (user_id, lesson_id, progress_percent, completed, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE progress_percent = GREATEST(progress_percent, VALUES(progress_percent)), completed = GREATEST(completed, VALUES(completed)), updated_at = CURRENT_TIMESTAMP",
        _ => "INSERT INTO lesson_progress (user_id, lesson_id, progress_percent, completed, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, lesson_id) DO UPDATE SET progress_percent = MAX(lesson_progress.progress_percent, excluded.progress_percent), completed = MAX(lesson_progress.completed, excluded.completed), updated_at = CURRENT_TIMESTAMP",
    };
    rullst::db::sqlx::query(upsert_sql).bind(user_id).bind(lesson_id).bind(progress_percent)
        .bind(i32::from(progress_percent == 100)).execute(&mut *transaction).await
        .map_err(|error| LearningError::Database(error.into()))?;
    transaction.commit().await.map_err(|error| LearningError::Database(error.into()))?;
    let progress = LessonProgress::for_learner(user_id, lesson_id).await?
        .ok_or(LearningError::NotFound("progress"))?;
    Ok(ProgressReceipt { applied: true, progress })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn owner_boundary_fails_before_database_access() {
        let context = UserContext::new("7", vec!["student".to_string()]);
        assert!(matches!(
            enroll(&context, 8, 1).await,
            Err(LearningError::Forbidden)
        ));
    }
}
"##;
