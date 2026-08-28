// Owner-derived completion and privacy-bounded certificate HTTP endpoints.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/controllers/completion_controller.rs",
        COMPLETION_CONTROLLER.to_string(),
    )]
}

const COMPLETION_CONTROLLER: &str = r##"use crate::services::completion_service::{
    CompletionError, derive_completion, revoke_certificate_at, verify_certificate,
};
use rullst::server::{Extension, IntoResponse, Json, Path, Response, StatusCode};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RevokeCertificatePayload {
    pub revocation_key: String,
    pub reason: String,
}

fn error_response(error: CompletionError) -> Response {
    match error {
        CompletionError::Forbidden => StatusCode::FORBIDDEN.into_response(),
        CompletionError::NotFound => StatusCode::NOT_FOUND.into_response(),
        CompletionError::Incomplete | CompletionError::InvalidState
        | CompletionError::IdempotencyConflict => StatusCode::CONFLICT.into_response(),
        CompletionError::InvalidField(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        CompletionError::Clock => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        CompletionError::Database(error) => {
            eprintln!("Course completion operation failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

fn unix_now() -> Result<i64, Response> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub async fn complete(
    Path(course_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
) -> Response {
    match derive_completion(&context, user_id, course_id).await {
        Ok(receipt) if receipt.applied => {
            (StatusCode::CREATED, Json(receipt)).into_response()
        }
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn verify(Path(certificate_key): Path<String>) -> Response {
    match verify_certificate(&certificate_key).await {
        Ok(verification) if verification.valid => Json(verification).into_response(),
        Ok(verification) => (StatusCode::GONE, Json(verification)).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn revoke(
    Path(certificate_key): Path<String>,
    Extension(context): Extension<UserContext>,
    Json(payload): Json<RevokeCertificatePayload>,
) -> Response {
    let revoked_at_epoch = match unix_now() {
        Ok(value) => value,
        Err(response) => return response,
    };
    match revoke_certificate_at(
        &context,
        &payload.revocation_key,
        &certificate_key,
        revoked_at_epoch,
        &payload.reason,
    )
    .await
    {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => error_response(error),
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::COMPLETION_CONTROLLER;

    #[test]
    fn completion_boundary_derives_subject_time_and_public_key_from_server_state() {
        assert!(COMPLETION_CONTROLLER.contains("Extension(user_id)"));
        assert!(COMPLETION_CONTROLLER.contains("Extension(context)"));
        assert!(COMPLETION_CONTROLLER.contains("Path(course_id)"));
        assert!(COMPLETION_CONTROLLER.contains("SystemTime::now"));
        assert!(COMPLETION_CONTROLLER.contains("StatusCode::GONE"));
        assert!(!COMPLETION_CONTROLLER.contains("pub subject_user_id"));
        assert!(!COMPLETION_CONTROLLER.contains("pub revoked_at_epoch"));
    }
}
