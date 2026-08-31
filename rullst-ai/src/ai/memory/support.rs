use super::{ChatMemoryError, MAX_MESSAGE_BYTES};

pub(super) fn validate_content(content: &str) -> Result<(), ChatMemoryError> {
    if content.trim().is_empty() || content.len() > MAX_MESSAGE_BYTES {
        Err(ChatMemoryError::InvalidContent)
    } else {
        Ok(())
    }
}

pub(super) fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or(0)
}
