//! Read-only HTTP adapter for the detached authoritative leaderboard.

pub(super) const GAMIFICATION_CONTROLLER: &str = r##"use crate::services::gamification_service::{self, GamificationError};
use crate::services::learning_service::LearningError;
use rullst::server::{Extension, IntoResponse, Json, Path, Query, Response, StatusCode};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub limit: Option<u32>,
}

fn access_error_response(error: LearningError) -> Response {
    match error {
        LearningError::NotFound(_) => StatusCode::NOT_FOUND.into_response(),
        LearningError::Forbidden => StatusCode::FORBIDDEN.into_response(),
        LearningError::InvalidField(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        LearningError::IdempotencyConflict => StatusCode::CONFLICT.into_response(),
        LearningError::Database(error) => {
            eprintln!("Leaderboard access check failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

fn error_response(error: GamificationError) -> Response {
    match error {
        GamificationError::Access(error) => access_error_response(error),
        GamificationError::NotFound | GamificationError::NotPublished => {
            StatusCode::NOT_FOUND.into_response()
        }
        GamificationError::AttemptLimit => StatusCode::TOO_MANY_REQUESTS.into_response(),
        GamificationError::IdempotencyConflict => StatusCode::CONFLICT.into_response(),
        GamificationError::UnsupportedSchemaVersion(_) | GamificationError::InvalidField(_) => {
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        GamificationError::InvalidIdentity => StatusCode::UNAUTHORIZED.into_response(),
        GamificationError::Clock => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        GamificationError::Database(error) => {
            eprintln!("Leaderboard operation failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

pub async fn leaderboard(
    Path((course_id, season_key)): Path<(i32, String)>,
    Query(query): Query<LeaderboardQuery>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
) -> Response {
    match gamification_service::leaderboard(
        &context,
        user_id,
        course_id,
        &season_key,
        query.limit.unwrap_or(25),
    ).await {
        Ok(entries) => Json(entries).into_response(),
        Err(error) => error_response(error),
    }
}
"##;
