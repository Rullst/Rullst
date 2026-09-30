use super::{AiError, ChatMemoryError, MAX_MESSAGE_BYTES};

pub(super) fn validate_content(content: &str) -> Result<(), ChatMemoryError> {
    if content.trim().is_empty() || content.len() > MAX_MESSAGE_BYTES {
        Err(ChatMemoryError::InvalidContent)
    } else {
        Ok(())
    }
}

/// A model answer outside the message bounds is a generation failure, not
/// invalid caller input.
pub(super) fn validate_response(response: &str) -> Result<(), AiError> {
    validate_content(response).map_err(|_| {
        AiError::ApiError(format!(
            "provider response must contain 1-{MAX_MESSAGE_BYTES} bytes"
        ))
    })
}

pub(super) fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or(0)
}
