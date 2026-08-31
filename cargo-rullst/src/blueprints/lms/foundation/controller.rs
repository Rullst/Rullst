pub(super) const FOUNDATION_CONTROLLER: &str = r##"use crate::pages::lms;
use crate::services::learning_service::{self, LearningError};
use rullst::server::{Extension, Form, IntoResponse, Path, Redirect, Response, StatusCode};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ProgressForm {
    pub progress_percent: i32,
    pub idempotency_key: String,
}

fn error_response(error: LearningError) -> Response {
    match error {
        LearningError::NotFound(_) => StatusCode::NOT_FOUND.into_response(),
        LearningError::Forbidden => StatusCode::FORBIDDEN.into_response(),
        LearningError::InvalidField(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        LearningError::IdempotencyConflict => StatusCode::CONFLICT.into_response(),
        LearningError::Database(error) => {
            eprintln!("Learning operation failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

pub async fn enroll(
    Path(course_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
) -> Response {
    match learning_service::enroll(&context, user_id, course_id).await {
        Ok(_) => Redirect::to(&format!("/courses/{course_id}")).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn play_lesson(
    Path(lesson_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    csrf: Option<Extension<rullst::security::CsrfToken>>,
    csp_nonce: Option<Extension<rullst::security::CspNonce>>,
) -> Response {
    let lesson = match learning_service::authorize_lesson(&context, user_id, lesson_id).await {
        Ok(lesson) => lesson,
        Err(error) => return error_response(error),
    };
    let progress = match crate::models::lesson_progress::LessonProgress::for_learner(
        user_id,
        lesson_id,
    ).await {
        Ok(progress) => progress.map_or(0, |value| value.progress_percent),
        Err(error) => return error_response(error.into()),
    };
    let csrf_token = csrf.as_ref().map(|Extension(value)| value.as_str()).unwrap_or_default();
    let nonce = csp_nonce.as_ref().map(|Extension(value)| value.as_str()).unwrap_or_default();
    let progress_key = format!("progress:{user_id}:{lesson_id}:next");
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
        csrf_token,
        &progress_key,
        nonce,
    ) {
        Ok(page) => rullst::response::Html(page).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub async fn record_progress(
    Path(lesson_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    Form(form): Form<ProgressForm>,
) -> Response {
    match learning_service::record_progress(
        &context,
        user_id,
        lesson_id,
        form.progress_percent,
        &form.idempotency_key,
    ).await {
        Ok(_) => Redirect::to(&format!("/lessons/{lesson_id}/play")).into_response(),
        Err(error) => error_response(error),
    }
}
"##;
