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

/// A fresh key for each rendered player, so the next save is a new event
/// while resubmitting the same page replays its save instead of repeating it.
fn new_progress_key(user_id: i32, lesson_id: i32) -> String {
    format!(
        "progress:{user_id}:{lesson_id}:{}",
        rullst::security::generate_csrf_token()
    )
}

/// The player's 25/50/100% buttons share one page key, so each requested
/// percentage is its own event: repeating a click replays it, while another
/// button (even on a page restored with the back button) still records.
fn progress_event_key(page_key: &str, progress_percent: i32) -> String {
    format!("{page_key}:{progress_percent}")
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
    let progress_key = new_progress_key(user_id, lesson_id);
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
        &progress_event_key(&form.idempotency_key, form.progress_percent),
    ).await {
        Ok(_) => Redirect::to(&format!("/lessons/{lesson_id}/play")).into_response(),
        Err(error) => error_response(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_render_and_requested_percentage_records_its_own_progress_event() {
        let page = new_progress_key(7, 1);
        assert!(page.starts_with("progress:7:1:"));
        assert_ne!(page, new_progress_key(7, 1));
        assert_eq!(progress_event_key(&page, 25), progress_event_key(&page, 25));
        assert_ne!(progress_event_key(&page, 25), progress_event_key(&page, 50));
        assert_ne!(progress_event_key(&page, 50), progress_event_key(&page, 100));
        // The learning service accepts at most 128 bytes of [A-Za-z0-9_.:-].
        let longest = progress_event_key(&new_progress_key(i32::MIN, i32::MIN), 100);
        assert!(longest.len() <= 128);
        assert!(longest.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-')));
    }
}
"##;
