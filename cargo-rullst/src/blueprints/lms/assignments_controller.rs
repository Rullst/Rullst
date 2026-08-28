// Authenticated HTTP boundary for assignment submission and grading.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/controllers/assignment_controller.rs",
        ASSIGNMENT_CONTROLLER.to_string(),
    )]
}

const ASSIGNMENT_CONTROLLER: &str = r##"use crate::services::assignment_grading_service::{
    AssignmentGradeError, AssignmentGradeInput, RubricScoreInput, grade_assignment_at,
};
use crate::services::assignment_grade_correction_service::{
    AssignmentGradeCorrectionError, AssignmentGradeCorrectionInput, correct_assignment_grade_at,
};
use crate::services::assignment_submission_service::{
    AssignmentSubmissionError, AssignmentSubmissionInput, submit_assignment_at,
};
use rullst::server::{Extension, IntoResponse, Json, Path, Response, StatusCode};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AssignmentSubmissionPayload {
    pub submission_key: String,
    pub content_text: String,
}

#[derive(Debug, Deserialize)]
pub struct RubricScorePayload {
    pub criterion_id: i32,
    pub points_awarded: i32,
    pub feedback: String,
}

#[derive(Debug, Deserialize)]
pub struct AssignmentGradePayload {
    pub grading_key: String,
    pub feedback: String,
    pub scores: Vec<RubricScorePayload>,
}

#[derive(Debug, Deserialize)]
pub struct AssignmentGradeCorrectionPayload {
    pub correction_key: String,
    pub reason: String,
    pub scores: Vec<RubricScorePayload>,
}

pub async fn submit(
    Path(assignment_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    Json(payload): Json<AssignmentSubmissionPayload>,
) -> Response {
    let now_epoch = match unix_now() { Ok(value) => value, Err(response) => return response };
    let input = AssignmentSubmissionInput {
        submission_key: payload.submission_key,
        assignment_id,
        subject_user_id: user_id,
        content_text: payload.content_text,
    };
    match submit_assignment_at(&context, input, now_epoch).await {
        Ok(receipt) if receipt.applied => (StatusCode::CREATED, Json(receipt)).into_response(),
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => submission_error(error),
    }
}

pub async fn grade(
    Path(submission_id): Path<i32>,
    Extension(context): Extension<UserContext>,
    Json(payload): Json<AssignmentGradePayload>,
) -> Response {
    let now_epoch = match unix_now() { Ok(value) => value, Err(response) => return response };
    let input = AssignmentGradeInput {
        grading_key: payload.grading_key,
        submission_id,
        feedback: payload.feedback,
        scores: payload.scores.into_iter().map(|score| RubricScoreInput {
            criterion_id: score.criterion_id,
            points_awarded: score.points_awarded,
            feedback: score.feedback,
        }).collect(),
    };
    match grade_assignment_at(&context, input, now_epoch).await {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => grade_error(error),
    }
}

pub async fn correct_grade(
    Path(assignment_grade_id): Path<i32>,
    Extension(context): Extension<UserContext>,
    Json(payload): Json<AssignmentGradeCorrectionPayload>,
) -> Response {
    let now_epoch = match unix_now() { Ok(value) => value, Err(response) => return response };
    let input = AssignmentGradeCorrectionInput {
        correction_key: payload.correction_key,
        assignment_grade_id,
        reason: payload.reason,
        scores: payload.scores.into_iter().map(|score| RubricScoreInput {
            criterion_id: score.criterion_id,
            points_awarded: score.points_awarded,
            feedback: score.feedback,
        }).collect(),
    };
    match correct_assignment_grade_at(&context, input, now_epoch).await {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => correction_error(error),
    }
}

fn submission_error(error: AssignmentSubmissionError) -> Response {
    match error {
        AssignmentSubmissionError::Access(_) | AssignmentSubmissionError::Forbidden => {
            StatusCode::FORBIDDEN.into_response()
        }
        AssignmentSubmissionError::NotFound => StatusCode::NOT_FOUND.into_response(),
        AssignmentSubmissionError::NotPublished | AssignmentSubmissionError::Deadline
        | AssignmentSubmissionError::AttemptLimit | AssignmentSubmissionError::IdempotencyConflict => {
            StatusCode::CONFLICT.into_response()
        }
        AssignmentSubmissionError::InvalidField(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        AssignmentSubmissionError::Clock => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        AssignmentSubmissionError::Database(error) => {
            eprintln!("Assignment submission failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

fn grade_error(error: AssignmentGradeError) -> Response {
    match error {
        AssignmentGradeError::Forbidden => StatusCode::FORBIDDEN.into_response(),
        AssignmentGradeError::NotFound => StatusCode::NOT_FOUND.into_response(),
        AssignmentGradeError::InvalidState | AssignmentGradeError::IdempotencyConflict => {
            StatusCode::CONFLICT.into_response()
        }
        AssignmentGradeError::InvalidField(_) | AssignmentGradeError::InvalidRubric
        | AssignmentGradeError::InvalidScore => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        AssignmentGradeError::Clock => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        AssignmentGradeError::Database(error) => {
            eprintln!("Assignment grading failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

fn correction_error(error: AssignmentGradeCorrectionError) -> Response {
    match error {
        AssignmentGradeCorrectionError::Forbidden => StatusCode::FORBIDDEN.into_response(),
        AssignmentGradeCorrectionError::NotFound => StatusCode::NOT_FOUND.into_response(),
        AssignmentGradeCorrectionError::IdempotencyConflict => StatusCode::CONFLICT.into_response(),
        AssignmentGradeCorrectionError::InvalidField(_)
        | AssignmentGradeCorrectionError::InvalidRubric
        | AssignmentGradeCorrectionError::InvalidScore => {
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        AssignmentGradeCorrectionError::Clock => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        AssignmentGradeCorrectionError::Database(error) => {
            eprintln!("Assignment grade correction failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

fn unix_now() -> Result<i64, Response> {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
"##;

#[cfg(test)]
mod tests {
    use super::ASSIGNMENT_CONTROLLER;

    #[test]
    fn assignment_http_derives_subject_resource_and_clock_from_server_state() {
        assert!(ASSIGNMENT_CONTROLLER.contains("Path(assignment_id)"));
        assert!(ASSIGNMENT_CONTROLLER.contains("Path(submission_id)"));
        assert!(ASSIGNMENT_CONTROLLER.contains("Path(assignment_grade_id)"));
        assert!(ASSIGNMENT_CONTROLLER.contains("Extension(user_id)"));
        assert!(ASSIGNMENT_CONTROLLER.contains("SystemTime::now"));
        assert!(!ASSIGNMENT_CONTROLLER.contains("pub subject_user_id"));
        assert!(!ASSIGNMENT_CONTROLLER.contains("pub now_epoch"));
    }
}
