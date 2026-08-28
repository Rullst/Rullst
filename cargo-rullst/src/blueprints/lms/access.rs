// Authenticated enrollment, entitlement and progress templates for the LMS starter.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/services/learning_service.rs",
            LEARNING_SERVICE.to_string(),
        ),
        (
            "src/services/mod.rs",
            "pub mod learning_service;\n".to_string(),
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
use rullst_security::{RbacGuard, UserContext};

#[derive(Debug)]
pub enum LearningError {
    NotFound(&'static str),
    Forbidden,
    InvalidProgress,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for LearningError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(resource) => write!(formatter, "{resource} not found"),
            Self::Forbidden => formatter.write_str("learning resource access denied"),
            Self::InvalidProgress => formatter.write_str("progress must be between 0 and 100"),
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

pub async fn enroll(
    user_id: i32,
    context: &UserContext,
    course_id: i32,
) -> Result<Enrollment, LearningError> {
    authorize_identity(context, user_id)?;
    if Course::find(course_id).await?.is_none() {
        return Err(LearningError::NotFound("course"));
    }

    let existing = Enrollment::query()
        .where_eq("user_id", user_id)
        .where_eq("course_id", course_id)
        .first()
        .await?;
    if let Some(mut enrollment) = existing {
        if enrollment.status != "active" {
            enrollment.status = "active".to_string();
            enrollment.save().await?;
        }
        return Ok(enrollment);
    }

    let mut enrollment = Enrollment {
        id: 0,
        user_id,
        course_id,
        status: "active".to_string(),
        created_at: String::new(),
        updated_at: String::new(),
    };
    if let Err(insert_error) = enrollment.save().await {
        // A concurrent identical request may have won the unique-key race.
        if let Some(winner) = Enrollment::active_for(user_id, course_id).await? {
            return Ok(winner);
        }
        return Err(LearningError::Database(insert_error));
    }
    Ok(enrollment)
}

pub async fn authorize_lesson(
    user_id: i32,
    context: &UserContext,
    lesson_id: i32,
) -> Result<Lesson, LearningError> {
    authorize_identity(context, user_id)?;
    let lesson = Lesson::find(lesson_id)
        .await?
        .ok_or(LearningError::NotFound("lesson"))?;
    let enrollment = Enrollment::active_for(user_id, lesson.course_id)
        .await?
        .ok_or(LearningError::Forbidden)?;
    RbacGuard::authorize_owner_or_role(context, &enrollment.user_id.to_string(), "admin")
        .map_err(|_| LearningError::Forbidden)?;
    Ok(lesson)
}

pub async fn record_progress(
    user_id: i32,
    context: &UserContext,
    lesson_id: i32,
    progress_percent: i32,
) -> Result<LessonProgress, LearningError> {
    if !(0..=100).contains(&progress_percent) {
        return Err(LearningError::InvalidProgress);
    }
    authorize_lesson(user_id, context, lesson_id).await?;

    let completed = i32::from(progress_percent == 100);
    let pool = rullst::db::Orm::pool()?;
    match rullst::db::Orm::driver()? {
        "postgres" => {
            rullst::db::sqlx::query(
                "INSERT INTO lesson_progress (user_id, lesson_id, progress_percent, completed, created_at, updated_at) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, lesson_id) DO UPDATE SET progress_percent = GREATEST(lesson_progress.progress_percent, EXCLUDED.progress_percent), completed = GREATEST(lesson_progress.completed, EXCLUDED.completed), updated_at = CURRENT_TIMESTAMP",
            )
            .bind(user_id)
            .bind(lesson_id)
            .bind(progress_percent)
            .bind(completed)
            .execute(pool)
            .await
            .map_err(|error| LearningError::Database(error.into()))?;
        }
        "mysql" => {
            rullst::db::sqlx::query(
                "INSERT INTO lesson_progress (user_id, lesson_id, progress_percent, completed, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE progress_percent = GREATEST(progress_percent, VALUES(progress_percent)), completed = GREATEST(completed, VALUES(completed)), updated_at = CURRENT_TIMESTAMP",
            )
            .bind(user_id)
            .bind(lesson_id)
            .bind(progress_percent)
            .bind(completed)
            .execute(pool)
            .await
            .map_err(|error| LearningError::Database(error.into()))?;
        }
        _ => {
            rullst::db::sqlx::query(
                "INSERT INTO lesson_progress (user_id, lesson_id, progress_percent, completed, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, lesson_id) DO UPDATE SET progress_percent = MAX(lesson_progress.progress_percent, excluded.progress_percent), completed = MAX(lesson_progress.completed, excluded.completed), updated_at = CURRENT_TIMESTAMP",
            )
            .bind(user_id)
            .bind(lesson_id)
            .bind(progress_percent)
            .bind(completed)
            .execute(pool)
            .await
            .map_err(|error| LearningError::Database(error.into()))?;
        }
    }

    LessonProgress::for_learner(user_id, lesson_id)
        .await?
        .ok_or(LearningError::NotFound("lesson progress"))
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
use rullst::server::{Extension, Form, IntoResponse, Path, Redirect, Response, StatusCode};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProgressDto {
    pub progress_percent: i32,
}

fn learning_error_response(error: LearningError) -> Response {
    let status = match &error {
        LearningError::NotFound(_) => StatusCode::NOT_FOUND,
        LearningError::Forbidden => StatusCode::FORBIDDEN,
        LearningError::InvalidProgress => StatusCode::UNPROCESSABLE_ENTITY,
        LearningError::Database(_) => {
            eprintln!("Learning operation failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE
        }
    };
    (status, status.canonical_reason().unwrap_or("Learning request failed")).into_response()
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
) -> Response {
    let lesson = match learning_service::authorize_lesson(user_id, &context, lesson_id).await {
        Ok(lesson) => lesson,
        Err(error) => return learning_error_response(error),
    };
    let progress = match LessonProgress::for_learner(user_id, lesson_id).await {
        Ok(progress) => progress.map_or(0, |value| value.progress_percent),
        Err(error) => return learning_error_response(LearningError::Database(error)),
    };
    rullst::response::Html(lms::video_player_snippet(
        &lesson.title,
        &lesson.video_url,
        lesson.id,
        progress,
        csrf.as_str(),
    ))
    .into_response()
}

pub async fn record_progress(
    Path(lesson_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    Form(payload): Form<ProgressDto>,
) -> Response {
    match learning_service::record_progress(
        user_id,
        &context,
        lesson_id,
        payload.progress_percent,
    )
    .await
    {
        Ok(progress) => rullst::response::Html(lms::progress_badge(progress.progress_percent))
            .into_response(),
        Err(error) => learning_error_response(error),
    }
}
"##;

const AUTH_MIDDLEWARE: &str = r##"use crate::models::user::User;
use rullst::server::{IntoResponse, Next, Redirect, Request, Response, StatusCode};
use rullst_security::UserContext;

pub async fn auth_middleware(mut request: Request, next: Next) -> Response {
    let Some(cookie) = rullst::auth::extract_session_cookie(request.headers()) else {
        return Redirect::to("/login").into_response();
    };
    let app_key = match rullst::auth::get_app_key() {
        Ok(key) => key,
        Err(error) => {
            eprintln!("Authentication key unavailable: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let Ok(user_id) = rullst::auth::decrypt_session(&cookie, &app_key) else {
        return Redirect::to("/login").into_response();
    };
    match User::find(user_id).await {
        Ok(Some(_)) => {
            request.extensions_mut().insert(user_id);
            request.extensions_mut().insert(UserContext::new(
                user_id.to_string(),
                vec!["student".to_string()],
            ));
            next.run(request).await
        }
        Ok(None) => Redirect::to("/login").into_response(),
        Err(error) => {
            eprintln!("Authentication user query failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}
"##;
