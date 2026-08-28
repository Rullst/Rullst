// Server-authoritative timed quiz attempt templates.

#[path = "assessment_controller.rs"]
mod assessment_controller;
#[path = "assignments.rs"]
mod assignments;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = vec![
        (
            "src/models/quiz_attempt_session.rs",
            QUIZ_ATTEMPT_SESSION.to_string(),
        ),
        (
            "src/services/assessment_timing_service.rs",
            ASSESSMENT_TIMING_SERVICE.to_string(),
        ),
    ];
    files.extend(assessment_controller::get_files());
    files.extend(assignments::get_files());
    files
}

const QUIZ_ATTEMPT_SESSION: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "quiz_attempt_sessions")]
pub struct QuizAttemptSession {
    pub id: i32,
    pub attempt_key: String,
    pub quiz_id: i32,
    pub actor_user_id: i32,
    pub subject_user_id: i32,
    pub ruleset_version: String,
    pub status: String,
    pub started_at_epoch: i64,
    pub expires_at_epoch: i64,
    pub presentation_json: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for QuizAttemptSession {
    fn nexus_table() -> &'static str { "quiz_attempt_sessions" }
    fn nexus_label() -> &'static str { "Quiz Attempt Sessions" }
    fn nexus_icon() -> &'static str { "⏱️" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "attempt_key", label: "Attempt Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "quiz_id", label: "Quiz", kind: FieldKind::ForeignKey { table: "quizzes", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Actor", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "status", label: "Status", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "started_at_epoch", label: "Started Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "expires_at_epoch", label: "Expires Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "presentation_json", label: "Persisted Presentation", kind: FieldKind::Json, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const ASSESSMENT_TIMING_SERVICE: &str = r##"use crate::services::learning_service::{LearningError, authorize_lesson};
use rullst_security::UserContext;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuizStartRequest {
    pub attempt_key: String,
    pub quiz_id: i32,
    pub subject_user_id: i32,
    pub ruleset_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuizStartReceipt {
    pub applied: bool,
    pub started_at_epoch: i64,
    pub expires_at_epoch: i64,
    pub presentation: QuizPresentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuizOptionOrder {
    pub question_id: i32,
    pub option_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuizPresentation {
    pub question_ids: Vec<i32>,
    pub option_orders: Vec<QuizOptionOrder>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPresentationV1 {
    schema_version: i32,
    seed: String,
    question_ids: Vec<i32>,
    option_orders: Vec<QuizOptionOrder>,
}

#[derive(Debug)]
pub enum QuizStartError {
    Access(LearningError),
    NotFound,
    NotPublished,
    UntimedQuiz,
    AttemptLimit,
    InvalidField(&'static str),
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for QuizStartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(error) => write!(formatter, "quiz start access error: {error}"),
            Self::NotFound => formatter.write_str("quiz not found"),
            Self::NotPublished => formatter.write_str("quiz is not published"),
            Self::UntimedQuiz => formatter.write_str("quiz does not require a timed start"),
            Self::AttemptLimit => formatter.write_str("quiz attempt limit reached"),
            Self::InvalidField(field) => write!(formatter, "invalid quiz start field: {field}"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "quiz start database error: {error}"),
        }
    }
}

impl std::error::Error for QuizStartError {}

impl From<LearningError> for QuizStartError {
    fn from(error: LearningError) -> Self { Self::Access(error) }
}

impl From<rullst_orm::Error> for QuizStartError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
        })
}

fn shuffled_ids(seed: &str, ruleset_version: &str, namespace: &str, ids: &[i32]) -> Vec<i32> {
    use std::hash::{Hash, Hasher};
    let mut keyed = ids
        .iter()
        .map(|id| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            (seed, ruleset_version, namespace, id).hash(&mut hasher);
            (hasher.finish(), *id)
        })
        .collect::<Vec<_>>();
    keyed.sort_unstable();
    keyed.into_iter().map(|entry| entry.1).collect()
}

fn build_presentation(
    seed: &str,
    ruleset_version: &str,
    question_ids: &[i32],
    options: &[(i32, i32)],
) -> Result<(QuizPresentation, String), QuizStartError> {
    if !valid_key(seed, 64) || question_ids.is_empty() || question_ids.len() > 100 {
        return Err(QuizStartError::InvalidField("quiz presentation"));
    }
    let mut options_by_question = BTreeMap::<i32, Vec<i32>>::new();
    for (question_id, option_id) in options {
        options_by_question
            .entry(*question_id)
            .or_default()
            .push(*option_id);
    }
    let question_ids = shuffled_ids(seed, ruleset_version, "question", question_ids);
    let mut option_orders = Vec::with_capacity(question_ids.len());
    for question_id in &question_ids {
        let option_ids = options_by_question
            .remove(question_id)
            .ok_or(QuizStartError::InvalidField("quiz presentation"))?;
        if option_ids.len() < 2 || option_ids.len() > 20 {
            return Err(QuizStartError::InvalidField("quiz presentation"));
        }
        option_orders.push(QuizOptionOrder {
            question_id: *question_id,
            option_ids: shuffled_ids(
                seed,
                ruleset_version,
                &format!("option:{question_id}"),
                &option_ids,
            ),
        });
    }
    if !options_by_question.is_empty() {
        return Err(QuizStartError::InvalidField("quiz presentation"));
    }
    let presentation = QuizPresentation {
        question_ids,
        option_orders,
    };
    let stored = StoredPresentationV1 {
        schema_version: 1,
        seed: seed.to_string(),
        question_ids: presentation.question_ids.clone(),
        option_orders: presentation.option_orders.clone(),
    };
    let json = serde_json::to_string(&stored)
        .map_err(|_| QuizStartError::InvalidField("quiz presentation"))?;
    Ok((presentation, json))
}

fn parse_presentation(json: &str) -> Result<QuizPresentation, QuizStartError> {
    let stored: StoredPresentationV1 = serde_json::from_str(json)
        .map_err(|_| QuizStartError::InvalidField("quiz presentation"))?;
    if stored.schema_version != 1 || !valid_key(&stored.seed, 64) {
        return Err(QuizStartError::InvalidField("quiz presentation"));
    }
    Ok(QuizPresentation {
        question_ids: stored.question_ids,
        option_orders: stored.option_orders,
    })
}

pub fn presentation_matches(
    json: &str,
    ruleset_version: &str,
    question_ids: &[i32],
    options: &[(i32, i32)],
) -> bool {
    let Ok(stored) = serde_json::from_str::<StoredPresentationV1>(json) else {
        return false;
    };
    if stored.schema_version != 1 || !valid_key(&stored.seed, 64) {
        return false;
    }
    let actual = QuizPresentation {
        question_ids: stored.question_ids,
        option_orders: stored.option_orders,
    };
    build_presentation(&stored.seed, ruleset_version, question_ids, options)
        .is_ok_and(|expected| expected.0 == actual)
}

fn unix_now() -> Result<i64, QuizStartError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| QuizStartError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| QuizStartError::Clock)
}

pub async fn start_quiz(
    context: &UserContext,
    request: QuizStartRequest,
) -> Result<QuizStartReceipt, QuizStartError> {
    start_quiz_at(context, request, unix_now()?).await
}

pub async fn start_quiz_at(
    context: &UserContext,
    request: QuizStartRequest,
    started_at_epoch: i64,
) -> Result<QuizStartReceipt, QuizStartError> {
    if request.quiz_id <= 0
        || request.subject_user_id <= 0
        || started_at_epoch <= 0
        || !valid_key(&request.attempt_key, 128)
        || !valid_key(&request.ruleset_version, 64)
    {
        return Err(QuizStartError::InvalidField("request"));
    }
    let actor_user_id = context
        .user_id
        .parse::<i32>()
        .map_err(|_| QuizStartError::InvalidField("actor identity"))?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let quiz_sql = match driver {
        "postgres" => "SELECT lesson_id, time_limit_seconds, max_attempts, ruleset_version, status FROM quizzes WHERE id = $1",
        _ => "SELECT lesson_id, time_limit_seconds, max_attempts, ruleset_version, status FROM quizzes WHERE id = ?",
    };
    let quiz = rullst::db::sqlx::query_as::<_, (i32, i32, i32, String, String)>(quiz_sql)
        .bind(request.quiz_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| QuizStartError::Database(error.into()))?
        .ok_or(QuizStartError::NotFound)?;
    authorize_lesson(request.subject_user_id, context, quiz.0).await?;
    if quiz.4 != "published" { return Err(QuizStartError::NotPublished); }
    if !(1..=86_400).contains(&quiz.1) { return Err(QuizStartError::UntimedQuiz); }
    if !(1..=100).contains(&quiz.2) || quiz.3 != request.ruleset_version {
        return Err(QuizStartError::InvalidField("quiz rules"));
    }
    let expires_at_epoch = started_at_epoch
        .checked_add(i64::from(quiz.1))
        .ok_or(QuizStartError::Clock)?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| QuizStartError::Database(error.into()))?;
    let replay_sql = match driver {
        "postgres" => "SELECT quiz_id, subject_user_id, ruleset_version, started_at_epoch, expires_at_epoch, presentation_json FROM quiz_attempt_sessions WHERE attempt_key = $1",
        _ => "SELECT quiz_id, subject_user_id, ruleset_version, started_at_epoch, expires_at_epoch, presentation_json FROM quiz_attempt_sessions WHERE attempt_key = ?",
    };
    if let Some(existing) = rullst::db::sqlx::query_as::<_, (i32, i32, String, i64, i64, String)>(replay_sql)
        .bind(&request.attempt_key)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| QuizStartError::Database(error.into()))?
    {
        if existing.0 != request.quiz_id
            || existing.1 != request.subject_user_id
            || existing.2 != request.ruleset_version
        {
            return Err(QuizStartError::InvalidField("attempt idempotency binding"));
        }
        transaction.commit().await.map_err(|error| QuizStartError::Database(error.into()))?;
        return Ok(QuizStartReceipt {
            applied: false,
            started_at_epoch: existing.3,
            expires_at_epoch: existing.4,
            presentation: parse_presentation(&existing.5)?,
        });
    }
    let count_sql = match driver {
        "postgres" => "SELECT COUNT(*) FROM quiz_attempt_sessions WHERE subject_user_id = $1 AND quiz_id = $2 AND ruleset_version = $3",
        _ => "SELECT COUNT(*) FROM quiz_attempt_sessions WHERE subject_user_id = ? AND quiz_id = ? AND ruleset_version = ?",
    };
    let attempts = rullst::db::sqlx::query_scalar::<_, i64>(count_sql)
        .bind(request.subject_user_id)
        .bind(request.quiz_id)
        .bind(&request.ruleset_version)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| QuizStartError::Database(error.into()))?;
    if attempts >= i64::from(quiz.2) { return Err(QuizStartError::AttemptLimit); }
    let question_sql = match driver {
        "postgres" => "SELECT id FROM quiz_questions WHERE quiz_id = $1 AND enabled = $2 ORDER BY position ASC, id ASC",
        _ => "SELECT id FROM quiz_questions WHERE quiz_id = ? AND enabled = ? ORDER BY position ASC, id ASC",
    };
    let question_ids = rullst::db::sqlx::query_scalar::<_, i32>(question_sql)
        .bind(request.quiz_id)
        .bind(1_i32)
        .fetch_all(&mut *transaction)
        .await
        .map_err(|error| QuizStartError::Database(error.into()))?;
    let option_sql = match driver {
        "postgres" => "SELECT quiz_options.question_id, quiz_options.id FROM quiz_options INNER JOIN quiz_questions ON quiz_questions.id = quiz_options.question_id WHERE quiz_questions.quiz_id = $1 AND quiz_questions.enabled = $2 ORDER BY quiz_options.position ASC, quiz_options.id ASC",
        _ => "SELECT quiz_options.question_id, quiz_options.id FROM quiz_options INNER JOIN quiz_questions ON quiz_questions.id = quiz_options.question_id WHERE quiz_questions.quiz_id = ? AND quiz_questions.enabled = ? ORDER BY quiz_options.position ASC, quiz_options.id ASC",
    };
    let options = rullst::db::sqlx::query_as::<_, (i32, i32)>(option_sql)
        .bind(request.quiz_id)
        .bind(1_i32)
        .fetch_all(&mut *transaction)
        .await
        .map_err(|error| QuizStartError::Database(error.into()))?;
    let seed = rullst::security::generate_csrf_token();
    let (presentation, presentation_json) =
        build_presentation(&seed, &request.ruleset_version, &question_ids, &options)?;
    let insert_sql = match driver {
        "postgres" => "INSERT INTO quiz_attempt_sessions (attempt_key, quiz_id, actor_user_id, subject_user_id, ruleset_version, status, started_at_epoch, expires_at_epoch, presentation_json, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO quiz_attempt_sessions (attempt_key, quiz_id, actor_user_id, subject_user_id, ruleset_version, status, started_at_epoch, expires_at_epoch, presentation_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(insert_sql)
        .bind(&request.attempt_key)
        .bind(request.quiz_id)
        .bind(actor_user_id)
        .bind(request.subject_user_id)
        .bind(&request.ruleset_version)
        .bind("started")
        .bind(started_at_epoch)
        .bind(expires_at_epoch)
        .bind(presentation_json)
        .execute(&mut *transaction)
        .await
        .map_err(|error| QuizStartError::Database(error.into()))?;
    transaction.commit().await.map_err(|error| QuizStartError::Database(error.into()))?;
    Ok(QuizStartReceipt { applied: true, started_at_epoch, expires_at_epoch, presentation })
}
"##;

#[cfg(test)]
mod tests {
    use super::ASSESSMENT_TIMING_SERVICE;

    #[test]
    fn timed_start_uses_server_epoch_and_exact_binding() {
        assert!(ASSESSMENT_TIMING_SERVICE.contains("started_at_epoch"));
        assert!(ASSESSMENT_TIMING_SERVICE.contains("expires_at_epoch"));
        assert!(ASSESSMENT_TIMING_SERVICE.contains("attempt idempotency binding"));
        assert!(ASSESSMENT_TIMING_SERVICE.contains("AttemptLimit"));
        assert!(ASSESSMENT_TIMING_SERVICE.contains("presentation_json"));
        assert!(ASSESSMENT_TIMING_SERVICE.contains("generate_csrf_token"));
        assert!(!ASSESSMENT_TIMING_SERVICE.contains("format!(\"SELECT"));
    }
}
