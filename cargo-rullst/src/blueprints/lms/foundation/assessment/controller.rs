//! HTTP adapter for the detached assessment service.

pub(super) const ASSESSMENT_CONTROLLER: &str = r##"use crate::services::assessment_service::{
    self, AssessmentError, QuizAnswerInput, QuizSubmission,
};
use crate::services::learning_service::LearningError;
use rullst::server::{Extension, IntoResponse, Json, Path, Response, StatusCode};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GradeQuizRequest {
    pub attempt_key: String,
    pub ruleset_version: String,
    pub answers: Vec<QuizAnswerInput>,
}

fn access_error_response(error: LearningError) -> Response {
    match error {
        LearningError::NotFound(_) => StatusCode::NOT_FOUND.into_response(),
        LearningError::Forbidden => StatusCode::FORBIDDEN.into_response(),
        LearningError::InvalidField(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        LearningError::IdempotencyConflict => StatusCode::CONFLICT.into_response(),
        LearningError::Database(error) => {
            eprintln!("Assessment access check failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

fn error_response(error: AssessmentError) -> Response {
    match error {
        AssessmentError::Access(error) => access_error_response(error),
        AssessmentError::NotFound | AssessmentError::NotPublished => {
            StatusCode::NOT_FOUND.into_response()
        }
        AssessmentError::AttemptLimit => StatusCode::TOO_MANY_REQUESTS.into_response(),
        AssessmentError::IdempotencyConflict => StatusCode::CONFLICT.into_response(),
        AssessmentError::InvalidField(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        AssessmentError::Clock => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        AssessmentError::Database(error) => {
            eprintln!("Assessment operation failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

pub async fn show(
    Path(quiz_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
) -> Response {
    match assessment_service::quiz_for_learner(&context, user_id, quiz_id).await {
        Ok(quiz) => Json(quiz).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn grade(
    Path(quiz_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    Json(request): Json<GradeQuizRequest>,
) -> Response {
    let submission = QuizSubmission {
        attempt_key: request.attempt_key,
        quiz_id,
        subject_user_id: user_id,
        ruleset_version: request.ruleset_version,
        answers: request.answers,
    };
    match assessment_service::grade_quiz(&context, submission).await {
        Ok(grade) => {
            let status = if grade.applied { StatusCode::CREATED } else { StatusCode::OK };
            (status, Json(grade)).into_response()
        }
        Err(error) => error_response(error),
    }
}
"##;
