// Transactional lesson progress, audit history and administrative corrections.

#[path = "completion.rs"]
mod completion;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = vec![(
        "src/services/progress_service.rs",
        PROGRESS_SERVICE.to_string(),
    )];
    files.extend(completion::get_files());
    files
}

const PROGRESS_SERVICE: &str = r##"use crate::models::lesson_progress::LessonProgress;
use crate::services::learning_service::{LearningError, authorize_lesson};
use crate::services::school_service;
use rullst_security::UserContext;

#[derive(Debug)]
pub enum ProgressError {
    Access(LearningError),
    InvalidField(&'static str),
    Database(rullst_orm::Error),
}

impl std::fmt::Display for ProgressError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(error) => write!(formatter, "progress access error: {error}"),
            Self::InvalidField(field) => write!(formatter, "invalid progress field: {field}"),
            Self::Database(error) => write!(formatter, "progress database error: {error}"),
        }
    }
}

impl std::error::Error for ProgressError {}

impl From<LearningError> for ProgressError {
    fn from(error: LearningError) -> Self {
        Self::Access(error)
    }
}

impl From<rullst_orm::Error> for ProgressError {
    fn from(error: rullst_orm::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug)]
pub struct ProgressChange {
    pub applied: bool,
    pub progress: LessonProgress,
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn actor_id(context: &UserContext) -> Result<i32, ProgressError> {
    context
        .user_id
        .parse::<i32>()
        .map_err(|_| ProgressError::InvalidField("actor identity"))
}

pub async fn record_progress(
    context: &UserContext,
    subject_user_id: i32,
    lesson_id: i32,
    progress_percent: i32,
    idempotency_key: &str,
) -> Result<ProgressChange, ProgressError> {
    if !(0..=100).contains(&progress_percent) || !valid_key(idempotency_key, 64) {
        return Err(ProgressError::InvalidField("progress submission"));
    }
    authorize_lesson(subject_user_id, context, lesson_id).await?;
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => ProgressError::Database(error),
            _ => ProgressError::InvalidField("school scope"),
        })?;
    let actor_user_id = actor_id(context)?;
    let event_key = format!("progress:{idempotency_key}:{progress_percent}");
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;

    let event_query = match driver {
        "postgres" => "SELECT id FROM lesson_progress_events WHERE event_key = $1",
        _ => "SELECT id FROM lesson_progress_events WHERE event_key = ?",
    };
    let replay = rullst::db::sqlx::query_scalar::<_, i32>(event_query)
        .bind(&event_key)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| ProgressError::Database(error.into()))?
        .is_some();
    let progress_query = match driver {
        "postgres" => "SELECT id, user_id, lesson_id, progress_percent, completed, created_at, updated_at FROM lesson_progress WHERE user_id = $1 AND lesson_id = $2",
        _ => "SELECT id, user_id, lesson_id, progress_percent, completed, created_at, updated_at FROM lesson_progress WHERE user_id = ? AND lesson_id = ?",
    };
    let current = rullst::db::sqlx::query_as::<_, LessonProgress>(progress_query)
        .bind(subject_user_id)
        .bind(lesson_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;
    if replay {
        transaction
            .commit()
            .await
            .map_err(|error| ProgressError::Database(error.into()))?;
        return current
            .map(|progress| ProgressChange { applied: false, progress })
            .ok_or(ProgressError::InvalidField("progress replay state"));
    }

    let previous_percent = current.as_ref().map_or(0, |value| value.progress_percent);
    let target_percent = previous_percent.max(progress_percent);
    if current.is_some() && target_percent == previous_percent {
        transaction
            .commit()
            .await
            .map_err(|error| ProgressError::Database(error.into()))?;
        return current
            .map(|progress| ProgressChange { applied: false, progress })
            .ok_or(ProgressError::InvalidField("progress state"));
    }
    let completed = i32::from(target_percent == 100);
    let upsert_sql = match driver {
        "postgres" => "INSERT INTO lesson_progress (user_id, lesson_id, progress_percent, completed, created_at, updated_at) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, lesson_id) DO UPDATE SET progress_percent = GREATEST(lesson_progress.progress_percent, EXCLUDED.progress_percent), completed = GREATEST(lesson_progress.completed, EXCLUDED.completed), updated_at = CURRENT_TIMESTAMP",
        "mysql" => "INSERT INTO lesson_progress (user_id, lesson_id, progress_percent, completed, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE progress_percent = GREATEST(progress_percent, VALUES(progress_percent)), completed = GREATEST(completed, VALUES(completed)), updated_at = CURRENT_TIMESTAMP",
        _ => "INSERT INTO lesson_progress (user_id, lesson_id, progress_percent, completed, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, lesson_id) DO UPDATE SET progress_percent = MAX(lesson_progress.progress_percent, excluded.progress_percent), completed = MAX(lesson_progress.completed, excluded.completed), updated_at = CURRENT_TIMESTAMP",
    };
    rullst::db::sqlx::query(upsert_sql)
        .bind(subject_user_id)
        .bind(lesson_id)
        .bind(target_percent)
        .bind(completed)
        .execute(&mut *transaction)
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;

    let history_sql = match driver {
        "postgres" => "INSERT INTO lesson_progress_events (event_key, actor_user_id, subject_user_id, lesson_id, previous_percent, current_percent, event_kind, reason, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (event_key) DO NOTHING",
        "mysql" => "INSERT IGNORE INTO lesson_progress_events (event_key, actor_user_id, subject_user_id, lesson_id, previous_percent, current_percent, event_kind, reason, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO lesson_progress_events (event_key, actor_user_id, subject_user_id, lesson_id, previous_percent, current_percent, event_kind, reason, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (event_key) DO NOTHING",
    };
    let inserted = rullst::db::sqlx::query(history_sql)
        .bind(&event_key)
        .bind(actor_user_id)
        .bind(subject_user_id)
        .bind(lesson_id)
        .bind(previous_percent)
        .bind(target_percent)
        .bind("advanced")
        .bind("")
        .execute(&mut *transaction)
        .await
        .map_err(|error| ProgressError::Database(error.into()))?
        .rows_affected()
        == 1;
    if !inserted {
        transaction
            .rollback()
            .await
            .map_err(|error| ProgressError::Database(error.into()))?;
        let progress = LessonProgress::for_learner(subject_user_id, lesson_id)
            .await?
            .ok_or(ProgressError::InvalidField("concurrent progress replay"))?;
        return Ok(ProgressChange { applied: false, progress });
    }

    if target_percent == 100 && previous_percent < 100 {
        let outbox_key = format!("lesson-completed:{subject_user_id}:{lesson_id}");
        let payload = serde_json::json!({
            "schema_version": 1,
            "actor_user_id": actor_user_id,
            "subject_user_id": subject_user_id,
            "lesson_id": lesson_id,
            "progress_event_key": event_key,
        })
        .to_string();
        let outbox_sql = match driver {
            "postgres" => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (event_key) DO NOTHING",
            "mysql" => "INSERT IGNORE INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            _ => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (event_key) DO NOTHING",
        };
        rullst::db::sqlx::query(outbox_sql)
            .bind(school_id)
            .bind(outbox_key)
            .bind("lesson_completed")
            .bind(subject_user_id)
            .bind(payload)
            .bind("pending")
            .bind("")
            .bind("")
            .bind("")
            .execute(&mut *transaction)
            .await
            .map_err(|error| ProgressError::Database(error.into()))?;
    }

    let progress = rullst::db::sqlx::query_as::<_, LessonProgress>(progress_query)
        .bind(subject_user_id)
        .bind(lesson_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;
    transaction
        .commit()
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;
    Ok(ProgressChange { applied: true, progress })
}

pub async fn correct_progress(
    context: &UserContext,
    correction_key: &str,
    subject_user_id: i32,
    lesson_id: i32,
    corrected_percent: i32,
    reason: &str,
) -> Result<ProgressChange, ProgressError> {
    if !context.has_role("admin") {
        return Err(ProgressError::Access(LearningError::Forbidden));
    }
    if !valid_key(correction_key, 64)
        || !(0..=100).contains(&corrected_percent)
        || !(8..=256).contains(&reason.len())
        || reason.chars().any(char::is_control)
    {
        return Err(ProgressError::InvalidField("progress correction"));
    }
    authorize_lesson(subject_user_id, context, lesson_id).await?;
    let actor_user_id = actor_id(context)?;
    let event_key = format!("progress-correction:{correction_key}");
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;
    let progress_query = match driver {
        "postgres" => "SELECT id, user_id, lesson_id, progress_percent, completed, created_at, updated_at FROM lesson_progress WHERE user_id = $1 AND lesson_id = $2",
        _ => "SELECT id, user_id, lesson_id, progress_percent, completed, created_at, updated_at FROM lesson_progress WHERE user_id = ? AND lesson_id = ?",
    };
    let current = rullst::db::sqlx::query_as::<_, LessonProgress>(progress_query)
        .bind(subject_user_id)
        .bind(lesson_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| ProgressError::Database(error.into()))?
        .ok_or(ProgressError::InvalidField("missing progress"))?;
    let replay_sql = match driver {
        "postgres" => "SELECT subject_user_id, lesson_id, current_percent FROM lesson_progress_events WHERE event_key = $1",
        _ => "SELECT subject_user_id, lesson_id, current_percent FROM lesson_progress_events WHERE event_key = ?",
    };
    let replay = rullst::db::sqlx::query_as::<_, (i32, i32, i32)>(replay_sql)
        .bind(&event_key)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;
    if let Some(binding) = replay {
        if binding != (subject_user_id, lesson_id, corrected_percent) {
            return Err(ProgressError::InvalidField("correction idempotency binding"));
        }
        transaction
            .commit()
            .await
            .map_err(|error| ProgressError::Database(error.into()))?;
        return Ok(ProgressChange { applied: false, progress: current });
    }

    let history_sql = match driver {
        "postgres" => "INSERT INTO lesson_progress_events (event_key, actor_user_id, subject_user_id, lesson_id, previous_percent, current_percent, event_kind, reason, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO lesson_progress_events (event_key, actor_user_id, subject_user_id, lesson_id, previous_percent, current_percent, event_kind, reason, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(history_sql)
        .bind(&event_key)
        .bind(actor_user_id)
        .bind(subject_user_id)
        .bind(lesson_id)
        .bind(current.progress_percent)
        .bind(corrected_percent)
        .bind("admin_correction")
        .bind(reason)
        .execute(&mut *transaction)
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;
    let update_sql = match driver {
        "postgres" => "UPDATE lesson_progress SET progress_percent = $1, completed = $2, updated_at = CURRENT_TIMESTAMP WHERE user_id = $3 AND lesson_id = $4",
        _ => "UPDATE lesson_progress SET progress_percent = ?, completed = ?, updated_at = CURRENT_TIMESTAMP WHERE user_id = ? AND lesson_id = ?",
    };
    rullst::db::sqlx::query(update_sql)
        .bind(corrected_percent)
        .bind(i32::from(corrected_percent == 100))
        .bind(subject_user_id)
        .bind(lesson_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;
    let progress = rullst::db::sqlx::query_as::<_, LessonProgress>(progress_query)
        .bind(subject_user_id)
        .bind(lesson_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;
    transaction
        .commit()
        .await
        .map_err(|error| ProgressError::Database(error.into()))?;
    Ok(ProgressChange { applied: true, progress })
}
"##;

#[cfg(test)]
mod tests {
    use super::PROGRESS_SERVICE;

    #[test]
    fn generated_progress_is_transactional_audited_and_idempotent() {
        assert!(PROGRESS_SERVICE.contains("lesson_progress_events"));
        assert!(PROGRESS_SERVICE.contains("lesson_completed"));
        assert!(PROGRESS_SERVICE.contains(".commit()"));
        assert!(PROGRESS_SERVICE.contains("correction idempotency binding"));
        assert!(!PROGRESS_SERVICE.contains("format!(\"UPDATE"));
    }
}
