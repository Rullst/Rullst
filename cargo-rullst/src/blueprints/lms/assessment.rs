// Authoritative server-side quiz grading templates.
pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/services/assessment_service.rs",
        ASSESSMENT_SERVICE.to_string(),
    )]
}

pub(super) const ASSESSMENT_SERVICE: &str = r##"use crate::services::assessment_timing_service::presentation_matches;
use crate::services::learning_service::{LearningError, authorize_lesson};
use crate::services::school_service;
use crate::services::score_service::invalidate_leaderboard_cache;
use rullst_security::UserContext;
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuizAnswerInput {
    pub question_id: i32,
    pub option_id: i32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuizSubmission {
    pub attempt_key: String,
    pub quiz_id: i32,
    pub subject_user_id: i32,
    pub ruleset_version: String,
    pub answers: Vec<QuizAnswerInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct QuizGrade {
    pub applied: bool,
    pub passed: bool,
    pub score_percent: i32,
    pub points_awarded: i32,
    pub max_points: i32,
}

#[derive(Debug)]
pub enum AssessmentError {
    Access(LearningError),
    NotFound,
    NotPublished,
    AttemptNotStarted,
    AttemptExpired,
    AttemptLimit,
    InvalidField(&'static str),
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for AssessmentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(error) => write!(formatter, "quiz access error: {error}"),
            Self::NotFound => formatter.write_str("quiz not found"),
            Self::NotPublished => formatter.write_str("quiz is not published"),
            Self::AttemptNotStarted => formatter.write_str("timed quiz attempt was not started"),
            Self::AttemptExpired => formatter.write_str("timed quiz attempt has expired"),
            Self::AttemptLimit => formatter.write_str("quiz attempt limit reached"),
            Self::InvalidField(field) => write!(formatter, "invalid quiz field: {field}"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "quiz database error: {error}"),
        }
    }
}

impl std::error::Error for AssessmentError {}

impl From<LearningError> for AssessmentError {
    fn from(error: LearningError) -> Self {
        Self::Access(error)
    }
}

impl From<rullst_orm::Error> for AssessmentError {
    fn from(error: rullst_orm::Error) -> Self {
        Self::Database(error)
    }
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn unix_now() -> Result<i64, AssessmentError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| AssessmentError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| AssessmentError::Clock)
}

pub async fn grade_quiz(
    context: &UserContext,
    submission: QuizSubmission,
) -> Result<QuizGrade, AssessmentError> {
    grade_quiz_at(context, submission, unix_now()?).await
}

pub async fn grade_quiz_at(
    context: &UserContext,
    submission: QuizSubmission,
    graded_at_epoch: i64,
) -> Result<QuizGrade, AssessmentError> {
    if submission.quiz_id <= 0
        || submission.subject_user_id <= 0
        || graded_at_epoch <= 0
        || !valid_key(&submission.attempt_key, 128)
        || !valid_key(&submission.ruleset_version, 64)
        || submission.answers.is_empty()
        || submission.answers.len() > 100
    {
        return Err(AssessmentError::InvalidField("submission"));
    }
    let actor_user_id = context
        .user_id
        .parse::<i32>()
        .map_err(|_| AssessmentError::InvalidField("actor identity"))?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let quiz_sql = match driver {
        "postgres" => "SELECT quizzes.lesson_id, quizzes.passing_score, quizzes.max_attempts, quizzes.time_limit_seconds, quizzes.ruleset_version, quizzes.status, lessons.course_id, quizzes.activity_id, quizzes.season_key, activities.activity_kind, activities.max_score, activities.ruleset_version, activities.evidence_sha256 FROM quizzes INNER JOIN lessons ON lessons.id = quizzes.lesson_id INNER JOIN activities ON activities.id = quizzes.activity_id WHERE quizzes.id = $1",
        _ => "SELECT quizzes.lesson_id, quizzes.passing_score, quizzes.max_attempts, quizzes.time_limit_seconds, quizzes.ruleset_version, quizzes.status, lessons.course_id, quizzes.activity_id, quizzes.season_key, activities.activity_kind, activities.max_score, activities.ruleset_version, activities.evidence_sha256 FROM quizzes INNER JOIN lessons ON lessons.id = quizzes.lesson_id INNER JOIN activities ON activities.id = quizzes.activity_id WHERE quizzes.id = ?",
    };
    let quiz = rullst::db::sqlx::query_as::<
        _,
        (i32, i32, i32, i32, String, String, i32, i32, String, String, i32, String, String),
    >(quiz_sql)
    .bind(submission.quiz_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| AssessmentError::Database(error.into()))?
    .ok_or(AssessmentError::NotFound)?;
    authorize_lesson(submission.subject_user_id, context, quiz.0).await?;
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => AssessmentError::Database(error),
            _ => AssessmentError::InvalidField("school scope"),
        })?;
    if quiz.5 != "published" {
        return Err(AssessmentError::NotPublished);
    }
    if quiz.4 != submission.ruleset_version
        || !(0..=100).contains(&quiz.1)
        || !(1..=100).contains(&quiz.2)
        || !(0..=86_400).contains(&quiz.3)
        || quiz.6 <= 0
        || quiz.7 <= 0
        || !valid_key(&quiz.8, 64)
        || quiz.9 != "quiz"
        || !(1..=1_000_000).contains(&quiz.10)
        || quiz.11 != quiz.4
        || quiz.12.len() != 64
        || !quiz.12.bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(AssessmentError::InvalidField("quiz rules"));
    }

    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    let replay_sql = match driver {
        "postgres" => "SELECT quiz_id, subject_user_id, ruleset_version, score_percent, points_awarded, max_points FROM quiz_attempts WHERE attempt_key = $1",
        _ => "SELECT quiz_id, subject_user_id, ruleset_version, score_percent, points_awarded, max_points FROM quiz_attempts WHERE attempt_key = ?",
    };
    let replay = rullst::db::sqlx::query_as::<_, (i32, i32, String, i32, i32, i32)>(replay_sql)
        .bind(&submission.attempt_key)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    if let Some(attempt) = replay {
        if attempt.0 != submission.quiz_id
            || attempt.1 != submission.subject_user_id
            || attempt.2 != submission.ruleset_version
        {
            return Err(AssessmentError::InvalidField("attempt idempotency binding"));
        }
        transaction
            .commit()
            .await
            .map_err(|error| AssessmentError::Database(error.into()))?;
        return Ok(QuizGrade {
            applied: false,
            passed: attempt.3 >= quiz.1,
            score_percent: attempt.3,
            points_awarded: attempt.4,
            max_points: attempt.5,
        });
    }

    let session_sql = match driver {
        "postgres" => "SELECT quiz_id, subject_user_id, ruleset_version, status, started_at_epoch, expires_at_epoch, presentation_json FROM quiz_attempt_sessions WHERE attempt_key = $1",
        _ => "SELECT quiz_id, subject_user_id, ruleset_version, status, started_at_epoch, expires_at_epoch, presentation_json FROM quiz_attempt_sessions WHERE attempt_key = ?",
    };
    let session = rullst::db::sqlx::query_as::<_, (i32, i32, String, String, i64, i64, String)>(
        session_sql,
    )
    .bind(&submission.attempt_key)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| AssessmentError::Database(error.into()))?;
    let timed_presentation = if let Some(value) = session {
        if value.0 != submission.quiz_id
            || value.1 != submission.subject_user_id
            || value.2 != submission.ruleset_version
            || value.3 != "started"
        {
            return Err(AssessmentError::InvalidField("timed attempt binding"));
        }
        if graded_at_epoch < value.4 {
            return Err(AssessmentError::InvalidField("attempt clock"));
        }
        if graded_at_epoch > value.5 {
            return Err(AssessmentError::AttemptExpired);
        }
        Some(value.6)
    } else {
        if quiz.3 != 0 {
            return Err(AssessmentError::AttemptNotStarted);
        }
        None
    };

    let count_sql = match driver {
        "postgres" => "SELECT COUNT(*) FROM quiz_attempts WHERE subject_user_id = $1 AND quiz_id = $2 AND ruleset_version = $3 AND status = $4",
        _ => "SELECT COUNT(*) FROM quiz_attempts WHERE subject_user_id = ? AND quiz_id = ? AND ruleset_version = ? AND status = ?",
    };
    let attempts = rullst::db::sqlx::query_scalar::<_, i64>(count_sql)
        .bind(submission.subject_user_id)
        .bind(submission.quiz_id)
        .bind(&submission.ruleset_version)
        .bind("graded")
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    if attempts >= i64::from(quiz.2) {
        return Err(AssessmentError::AttemptLimit);
    }

    let question_sql = match driver {
        "postgres" => "SELECT id, points FROM quiz_questions WHERE quiz_id = $1 AND enabled = $2 ORDER BY position ASC, id ASC",
        _ => "SELECT id, points FROM quiz_questions WHERE quiz_id = ? AND enabled = ? ORDER BY position ASC, id ASC",
    };
    let questions = rullst::db::sqlx::query_as::<_, (i32, i32)>(question_sql)
        .bind(submission.quiz_id)
        .bind(1_i32)
        .fetch_all(&mut *transaction)
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    if questions.is_empty() || questions.len() != submission.answers.len() {
        return Err(AssessmentError::InvalidField("answer coverage"));
    }
    let option_sql = match driver {
        "postgres" => "SELECT quiz_options.id, quiz_options.question_id, quiz_options.is_correct FROM quiz_options INNER JOIN quiz_questions ON quiz_questions.id = quiz_options.question_id WHERE quiz_questions.quiz_id = $1 AND quiz_questions.enabled = $2",
        _ => "SELECT quiz_options.id, quiz_options.question_id, quiz_options.is_correct FROM quiz_options INNER JOIN quiz_questions ON quiz_questions.id = quiz_options.question_id WHERE quiz_questions.quiz_id = ? AND quiz_questions.enabled = ?",
    };
    let options = rullst::db::sqlx::query_as::<_, (i32, i32, i32)>(option_sql)
        .bind(submission.quiz_id)
        .bind(1_i32)
        .fetch_all(&mut *transaction)
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    if timed_presentation.as_ref().is_some_and(|presentation| {
        !presentation_matches(
            presentation,
            &submission.ruleset_version,
            &questions.iter().map(|question| question.0).collect::<Vec<_>>(),
            &options.iter().map(|option| (option.1, option.0)).collect::<Vec<_>>(),
        )
    }) {
        return Err(AssessmentError::InvalidField("quiz presentation binding"));
    }
    let option_map = options
        .iter()
        .map(|option| (option.0, (option.1, option.2 == 1)))
        .collect::<BTreeMap<_, _>>();
    for question in &questions {
        if options
            .iter()
            .filter(|option| option.1 == question.0 && option.2 == 1)
            .count()
            != 1
        {
            return Err(AssessmentError::InvalidField("correct option invariant"));
        }
    }
    let answer_map = submission
        .answers
        .iter()
        .map(|answer| (answer.question_id, answer.option_id))
        .collect::<BTreeMap<_, _>>();
    if answer_map.len() != questions.len() {
        return Err(AssessmentError::InvalidField("duplicate answer"));
    }
    let mut points_awarded = 0_i32;
    let mut max_points = 0_i32;
    let mut graded_answers = Vec::with_capacity(questions.len());
    for (question_id, points) in &questions {
        if !(1..=100_000).contains(points) {
            return Err(AssessmentError::InvalidField("question points"));
        }
        max_points = max_points
            .checked_add(*points)
            .ok_or(AssessmentError::InvalidField("maximum points"))?;
        let option_id = answer_map
            .get(question_id)
            .ok_or(AssessmentError::InvalidField("missing answer"))?;
        let selected = option_map
            .get(option_id)
            .ok_or(AssessmentError::InvalidField("unknown option"))?;
        if selected.0 != *question_id {
            return Err(AssessmentError::InvalidField("option ownership"));
        }
        let awarded = if selected.1 { *points } else { 0 };
        points_awarded = points_awarded
            .checked_add(awarded)
            .ok_or(AssessmentError::InvalidField("awarded points"))?;
        graded_answers.push((*question_id, *option_id, selected.1, awarded));
    }
    let score_percent = points_awarded
        .checked_mul(100)
        .ok_or(AssessmentError::InvalidField("score calculation"))?
        / max_points;
    if max_points != quiz.10 {
        return Err(AssessmentError::InvalidField("activity maximum score"));
    }
    let attempt_sql = match driver {
        "postgres" => "INSERT INTO quiz_attempts (attempt_key, quiz_id, actor_user_id, subject_user_id, ruleset_version, status, score_percent, points_awarded, max_points, graded_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO quiz_attempts (attempt_key, quiz_id, actor_user_id, subject_user_id, ruleset_version, status, score_percent, points_awarded, max_points, graded_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(attempt_sql)
        .bind(&submission.attempt_key)
        .bind(submission.quiz_id)
        .bind(actor_user_id)
        .bind(submission.subject_user_id)
        .bind(&submission.ruleset_version)
        .bind("graded")
        .bind(score_percent)
        .bind(points_awarded)
        .bind(max_points)
        .bind(graded_at_epoch)
        .execute(&mut *transaction)
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    let attempt_id = rullst::db::sqlx::query_scalar::<_, i32>(
        match driver {
            "postgres" => "SELECT id FROM quiz_attempts WHERE attempt_key = $1",
            _ => "SELECT id FROM quiz_attempts WHERE attempt_key = ?",
        },
    )
    .bind(&submission.attempt_key)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| AssessmentError::Database(error.into()))?;
    let answer_sql = match driver {
        "postgres" => "INSERT INTO quiz_answers (attempt_id, question_id, option_id, correct, points_awarded, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO quiz_answers (attempt_id, question_id, option_id, correct, points_awarded, created_at, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    for answer in graded_answers {
        rullst::db::sqlx::query(answer_sql)
            .bind(attempt_id)
            .bind(answer.0)
            .bind(answer.1)
            .bind(i32::from(answer.2))
            .bind(answer.3)
            .execute(&mut *transaction)
            .await
            .map_err(|error| AssessmentError::Database(error.into()))?;
    }

    let score_idempotency_key = format!("quiz:{}", submission.attempt_key);
    let score_sql = match driver {
        "postgres" => "INSERT INTO score_events (idempotency_key, schema_version, origin, actor_user_id, subject_user_id, course_id, activity_id, attempt_key, points, max_score, occurred_at, ruleset_version, evidence_sha256, created_at, updated_at) VALUES ($1, 2, $2, $3, $4, $5, $6, $7, $8, $9, CURRENT_TIMESTAMP, $10, $11, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO score_events (idempotency_key, schema_version, origin, actor_user_id, subject_user_id, course_id, activity_id, attempt_key, points, max_score, occurred_at, ruleset_version, evidence_sha256, created_at, updated_at) VALUES (?, 2, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(score_sql)
        .bind(&score_idempotency_key)
        .bind("quiz")
        .bind(actor_user_id)
        .bind(submission.subject_user_id)
        .bind(quiz.6)
        .bind(quiz.7)
        .bind(&submission.attempt_key)
        .bind(points_awarded)
        .bind(max_points)
        .bind(&submission.ruleset_version)
        .bind(&quiz.12)
        .execute(&mut *transaction)
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    let leaderboard_sql = match driver {
        "postgres" => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id, season_key) DO UPDATE SET score = leaderboard_entries.score + EXCLUDED.score, updated_at = CURRENT_TIMESTAMP",
        "mysql" => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE score = score + VALUES(score), updated_at = CURRENT_TIMESTAMP",
        _ => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id, season_key) DO UPDATE SET score = leaderboard_entries.score + excluded.score, updated_at = CURRENT_TIMESTAMP",
    };
    rullst::db::sqlx::query(leaderboard_sql)
        .bind(submission.subject_user_id)
        .bind(quiz.6)
        .bind(&quiz.8)
        .bind(points_awarded)
        .execute(&mut *transaction)
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;

    let score_event_key = format!("score:{score_idempotency_key}");
    let score_payload = serde_json::json!({
        "schema_version": 2,
        "idempotency_key": &score_idempotency_key,
        "origin": "quiz",
        "actor_user_id": actor_user_id,
        "subject_user_id": submission.subject_user_id,
        "course_id": quiz.6,
        "activity_id": quiz.7,
        "attempt_key": &submission.attempt_key,
        "points": points_awarded,
        "max_score": max_points,
        "ruleset_version": &submission.ruleset_version,
        "season_key": &quiz.8,
        "evidence_sha256": &quiz.12,
    })
    .to_string();
    let event_key = format!("quiz-graded:{}", submission.attempt_key);
    let payload = serde_json::json!({
        "schema_version": 1,
        "actor_user_id": actor_user_id,
        "subject_user_id": submission.subject_user_id,
        "quiz_id": submission.quiz_id,
        "attempt_key": submission.attempt_key,
        "ruleset_version": submission.ruleset_version,
        "score_percent": score_percent,
        "passed": score_percent >= quiz.1,
    })
    .to_string();
    let outbox_sql = match driver {
        "postgres" => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(outbox_sql)
        .bind(school_id)
        .bind(score_event_key)
        .bind("score_recorded")
        .bind(submission.subject_user_id)
        .bind(score_payload)
        .bind("pending")
        .bind("")
        .bind("")
        .bind("")
        .execute(&mut *transaction)
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    rullst::db::sqlx::query(outbox_sql)
        .bind(school_id)
        .bind(event_key)
        .bind("quiz_graded")
        .bind(submission.subject_user_id)
        .bind(payload)
        .bind("pending")
        .bind("")
        .bind("")
        .bind("")
        .execute(&mut *transaction)
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    if timed_presentation.is_some() {
        let session_update_sql = match driver {
            "postgres" => "UPDATE quiz_attempt_sessions SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE attempt_key = $2 AND quiz_id = $3 AND subject_user_id = $4 AND ruleset_version = $5 AND status = $6",
            _ => "UPDATE quiz_attempt_sessions SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE attempt_key = ? AND quiz_id = ? AND subject_user_id = ? AND ruleset_version = ? AND status = ?",
        };
        let update = rullst::db::sqlx::query(session_update_sql)
            .bind("graded")
            .bind(&submission.attempt_key)
            .bind(submission.quiz_id)
            .bind(submission.subject_user_id)
            .bind(&submission.ruleset_version)
            .bind("started")
            .execute(&mut *transaction)
            .await
            .map_err(|error| AssessmentError::Database(error.into()))?;
        if update.rows_affected() != 1 {
            return Err(AssessmentError::InvalidField("timed attempt state"));
        }
    }
    transaction
        .commit()
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    let _ = invalidate_leaderboard_cache(context, quiz.6, &quiz.8).await;
    Ok(QuizGrade {
        applied: true,
        passed: score_percent >= quiz.1,
        score_percent,
        points_awarded,
        max_points,
    })
}
"##;
