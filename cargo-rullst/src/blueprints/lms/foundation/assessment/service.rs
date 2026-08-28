//! Server-authoritative assessment service emitted by the detached profile.

pub(super) const ASSESSMENT_SERVICE: &str = r##"use crate::services::learning_service::{LearningError, authorize_lesson};
use rullst_security::UserContext;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct QuizOptionView {
    pub id: i32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct QuizQuestionView {
    pub id: i32,
    pub prompt: String,
    pub points: i32,
    pub options: Vec<QuizOptionView>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct QuizPresentation {
    pub id: i32,
    pub title: String,
    pub passing_score: i32,
    pub max_attempts: i32,
    pub ruleset_version: String,
    pub questions: Vec<QuizQuestionView>,
}

#[derive(Debug)]
pub enum AssessmentError {
    Access(LearningError),
    NotFound,
    NotPublished,
    AttemptLimit,
    IdempotencyConflict,
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
            Self::AttemptLimit => formatter.write_str("quiz attempt limit reached"),
            Self::IdempotencyConflict => formatter.write_str("quiz idempotency conflict"),
            Self::InvalidField(field) => write!(formatter, "invalid quiz field: {field}"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "quiz database error: {error}"),
        }
    }
}

impl std::error::Error for AssessmentError {}

impl From<LearningError> for AssessmentError {
    fn from(error: LearningError) -> Self { Self::Access(error) }
}

impl From<rullst_orm::Error> for AssessmentError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

#[derive(Debug)]
struct QuizRules {
    lesson_id: i32,
    title: String,
    passing_score: i32,
    max_attempts: i32,
    ruleset_version: String,
    status: String,
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
    })
}

fn unix_now() -> Result<i64, AssessmentError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| AssessmentError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| AssessmentError::Clock)
}

async fn quiz_rules(quiz_id: i32) -> Result<QuizRules, AssessmentError> {
    if quiz_id <= 0 { return Err(AssessmentError::InvalidField("quiz")); }
    let sql = match rullst::db::Orm::driver()? {
        "postgres" => "SELECT lesson_id, title, passing_score, max_attempts, ruleset_version, status FROM quizzes WHERE id = $1",
        _ => "SELECT lesson_id, title, passing_score, max_attempts, ruleset_version, status FROM quizzes WHERE id = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (i32, String, i32, i32, String, String)>(sql)
        .bind(quiz_id)
        .fetch_optional(rullst::db::Orm::pool()?)
        .await
        .map_err(|error| AssessmentError::Database(error.into()))?
        .ok_or(AssessmentError::NotFound)?;
    Ok(QuizRules {
        lesson_id: row.0,
        title: row.1,
        passing_score: row.2,
        max_attempts: row.3,
        ruleset_version: row.4,
        status: row.5,
    })
}

fn validate_rules(rules: &QuizRules) -> Result<(), AssessmentError> {
    if rules.status != "published" { return Err(AssessmentError::NotPublished); }
    if !(0..=100).contains(&rules.passing_score)
        || !(1..=100).contains(&rules.max_attempts)
        || !valid_key(&rules.ruleset_version, 64)
    {
        return Err(AssessmentError::InvalidField("quiz rules"));
    }
    Ok(())
}

pub async fn quiz_for_learner(
    context: &UserContext,
    user_id: i32,
    quiz_id: i32,
) -> Result<QuizPresentation, AssessmentError> {
    let rules = quiz_rules(quiz_id).await?;
    validate_rules(&rules)?;
    authorize_lesson(context, user_id, rules.lesson_id).await?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let question_sql = match driver {
        "postgres" => "SELECT id, prompt, points FROM quiz_questions WHERE quiz_id = $1 AND enabled = $2 ORDER BY position ASC, id ASC",
        _ => "SELECT id, prompt, points FROM quiz_questions WHERE quiz_id = ? AND enabled = ? ORDER BY position ASC, id ASC",
    };
    let questions = rullst::db::sqlx::query_as::<_, (i32, String, i32)>(question_sql)
        .bind(quiz_id).bind(1_i32).fetch_all(pool).await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    if questions.is_empty() { return Err(AssessmentError::InvalidField("quiz questions")); }
    let option_sql = match driver {
        "postgres" => "SELECT quiz_options.question_id, quiz_options.id, quiz_options.label FROM quiz_options INNER JOIN quiz_questions ON quiz_questions.id = quiz_options.question_id WHERE quiz_questions.quiz_id = $1 AND quiz_questions.enabled = $2 ORDER BY quiz_questions.position ASC, quiz_options.position ASC, quiz_options.id ASC",
        _ => "SELECT quiz_options.question_id, quiz_options.id, quiz_options.label FROM quiz_options INNER JOIN quiz_questions ON quiz_questions.id = quiz_options.question_id WHERE quiz_questions.quiz_id = ? AND quiz_questions.enabled = ? ORDER BY quiz_questions.position ASC, quiz_options.position ASC, quiz_options.id ASC",
    };
    let options = rullst::db::sqlx::query_as::<_, (i32, i32, String)>(option_sql)
        .bind(quiz_id).bind(1_i32).fetch_all(pool).await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    let mut options_by_question = BTreeMap::<i32, Vec<QuizOptionView>>::new();
    for (question_id, id, label) in options {
        options_by_question.entry(question_id).or_default().push(QuizOptionView { id, label });
    }
    let mut presentation = Vec::with_capacity(questions.len());
    for (id, prompt, points) in questions {
        if !(1..=100_000).contains(&points) {
            return Err(AssessmentError::InvalidField("question points"));
        }
        let options = options_by_question.remove(&id).unwrap_or_default();
        if options.len() < 2 { return Err(AssessmentError::InvalidField("question options")); }
        presentation.push(QuizQuestionView { id, prompt, points, options });
    }
    Ok(QuizPresentation {
        id: quiz_id,
        title: rules.title,
        passing_score: rules.passing_score,
        max_attempts: rules.max_attempts,
        ruleset_version: rules.ruleset_version,
        questions: presentation,
    })
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
    let actor_user_id = context.user_id.parse::<i32>()
        .map_err(|_| AssessmentError::InvalidField("actor identity"))?;
    let rules = quiz_rules(submission.quiz_id).await?;
    validate_rules(&rules)?;
    authorize_lesson(context, submission.subject_user_id, rules.lesson_id).await?;
    if rules.ruleset_version != submission.ruleset_version {
        return Err(AssessmentError::InvalidField("ruleset version"));
    }

    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool.begin().await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    let replay_sql = match driver {
        "postgres" => "SELECT quiz_id, subject_user_id, ruleset_version, score_percent, points_awarded, max_points FROM quiz_attempts WHERE attempt_key = $1",
        _ => "SELECT quiz_id, subject_user_id, ruleset_version, score_percent, points_awarded, max_points FROM quiz_attempts WHERE attempt_key = ?",
    };
    if let Some(attempt) = rullst::db::sqlx::query_as::<_, (i32, i32, String, i32, i32, i32)>(replay_sql)
        .bind(&submission.attempt_key).fetch_optional(&mut *transaction).await
        .map_err(|error| AssessmentError::Database(error.into()))?
    {
        if attempt.0 != submission.quiz_id || attempt.1 != submission.subject_user_id
            || attempt.2 != submission.ruleset_version
        {
            return Err(AssessmentError::IdempotencyConflict);
        }
        transaction.commit().await
            .map_err(|error| AssessmentError::Database(error.into()))?;
        return Ok(QuizGrade {
            applied: false,
            passed: attempt.3 >= rules.passing_score,
            score_percent: attempt.3,
            points_awarded: attempt.4,
            max_points: attempt.5,
        });
    }

    let lock_sql = match driver {
        "postgres" => "SELECT id FROM quizzes WHERE id = $1 FOR UPDATE",
        "mysql" => "SELECT id FROM quizzes WHERE id = ? FOR UPDATE",
        _ => "SELECT id FROM quizzes WHERE id = ?",
    };
    rullst::db::sqlx::query_scalar::<_, i32>(lock_sql).bind(submission.quiz_id)
        .fetch_one(&mut *transaction).await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    let count_sql = match driver {
        "postgres" => "SELECT COUNT(*) FROM quiz_attempts WHERE subject_user_id = $1 AND quiz_id = $2 AND ruleset_version = $3 AND status = $4",
        _ => "SELECT COUNT(*) FROM quiz_attempts WHERE subject_user_id = ? AND quiz_id = ? AND ruleset_version = ? AND status = ?",
    };
    let attempts = rullst::db::sqlx::query_scalar::<_, i64>(count_sql)
        .bind(submission.subject_user_id).bind(submission.quiz_id)
        .bind(&submission.ruleset_version).bind("graded")
        .fetch_one(&mut *transaction).await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    if attempts >= i64::from(rules.max_attempts) { return Err(AssessmentError::AttemptLimit); }

    let question_sql = match driver {
        "postgres" => "SELECT id, points FROM quiz_questions WHERE quiz_id = $1 AND enabled = $2 ORDER BY position ASC, id ASC",
        _ => "SELECT id, points FROM quiz_questions WHERE quiz_id = ? AND enabled = ? ORDER BY position ASC, id ASC",
    };
    let questions = rullst::db::sqlx::query_as::<_, (i32, i32)>(question_sql)
        .bind(submission.quiz_id).bind(1_i32).fetch_all(&mut *transaction).await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    if questions.is_empty() || questions.len() != submission.answers.len() {
        return Err(AssessmentError::InvalidField("answer coverage"));
    }
    let option_sql = match driver {
        "postgres" => "SELECT quiz_options.id, quiz_options.question_id, quiz_options.is_correct FROM quiz_options INNER JOIN quiz_questions ON quiz_questions.id = quiz_options.question_id WHERE quiz_questions.quiz_id = $1 AND quiz_questions.enabled = $2",
        _ => "SELECT quiz_options.id, quiz_options.question_id, quiz_options.is_correct FROM quiz_options INNER JOIN quiz_questions ON quiz_questions.id = quiz_options.question_id WHERE quiz_questions.quiz_id = ? AND quiz_questions.enabled = ?",
    };
    let options = rullst::db::sqlx::query_as::<_, (i32, i32, i32)>(option_sql)
        .bind(submission.quiz_id).bind(1_i32).fetch_all(&mut *transaction).await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    let option_map = options.iter()
        .map(|option| (option.0, (option.1, option.2 == 1)))
        .collect::<BTreeMap<_, _>>();
    for question in &questions {
        if options.iter().filter(|option| option.1 == question.0 && option.2 == 1).count() != 1 {
            return Err(AssessmentError::InvalidField("correct option invariant"));
        }
    }
    let answer_map = submission.answers.iter()
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
        max_points = max_points.checked_add(*points)
            .ok_or(AssessmentError::InvalidField("maximum points"))?;
        let option_id = answer_map.get(question_id)
            .ok_or(AssessmentError::InvalidField("missing answer"))?;
        let selected = option_map.get(option_id)
            .ok_or(AssessmentError::InvalidField("unknown option"))?;
        if selected.0 != *question_id {
            return Err(AssessmentError::InvalidField("option ownership"));
        }
        let awarded = if selected.1 { *points } else { 0 };
        points_awarded = points_awarded.checked_add(awarded)
            .ok_or(AssessmentError::InvalidField("awarded points"))?;
        graded_answers.push((*question_id, *option_id, selected.1, awarded));
    }
    let score_percent = points_awarded.checked_mul(100)
        .ok_or(AssessmentError::InvalidField("score calculation"))? / max_points;
    let attempt_sql = match driver {
        "postgres" => "INSERT INTO quiz_attempts (attempt_key, quiz_id, actor_user_id, subject_user_id, ruleset_version, status, score_percent, points_awarded, max_points, graded_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO quiz_attempts (attempt_key, quiz_id, actor_user_id, subject_user_id, ruleset_version, status, score_percent, points_awarded, max_points, graded_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(attempt_sql).bind(&submission.attempt_key)
        .bind(submission.quiz_id).bind(actor_user_id).bind(submission.subject_user_id)
        .bind(&submission.ruleset_version).bind("graded").bind(score_percent)
        .bind(points_awarded).bind(max_points).bind(graded_at_epoch)
        .execute(&mut *transaction).await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    let attempt_id_sql = match driver {
        "postgres" => "SELECT id FROM quiz_attempts WHERE attempt_key = $1",
        _ => "SELECT id FROM quiz_attempts WHERE attempt_key = ?",
    };
    let attempt_id = rullst::db::sqlx::query_scalar::<_, i32>(attempt_id_sql)
        .bind(&submission.attempt_key).fetch_one(&mut *transaction).await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    let answer_sql = match driver {
        "postgres" => "INSERT INTO quiz_answers (attempt_id, question_id, option_id, correct, points_awarded, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO quiz_answers (attempt_id, question_id, option_id, correct, points_awarded, created_at, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    for (question_id, option_id, correct, awarded) in graded_answers {
        rullst::db::sqlx::query(answer_sql).bind(attempt_id).bind(question_id).bind(option_id)
            .bind(i32::from(correct)).bind(awarded).execute(&mut *transaction).await
            .map_err(|error| AssessmentError::Database(error.into()))?;
    }
    transaction.commit().await
        .map_err(|error| AssessmentError::Database(error.into()))?;
    Ok(QuizGrade {
        applied: true,
        passed: score_percent >= rules.passing_score,
        score_percent,
        points_awarded,
        max_points,
    })
}
"##;
