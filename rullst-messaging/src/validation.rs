//! Shared bounded grammars for broker-independent identifiers and metadata.

use crate::{MessagingError, Result};

pub(crate) const MAX_NAMESPACE_BYTES: usize = 128;
pub(crate) const MAX_TOPIC_BYTES: usize = 255;
pub(crate) const MAX_GROUP_BYTES: usize = 128;
pub(crate) const MAX_CONSUMER_BYTES: usize = 128;
pub(crate) const MAX_EVENT_KIND_BYTES: usize = 128;
pub(crate) const MAX_IDEMPOTENCY_BYTES: usize = 255;
pub(crate) const MAX_CONTENT_TYPE_BYTES: usize = 127;
pub(crate) const MAX_FAILURE_CODE_BYTES: usize = 128;
pub(crate) const MAX_HEADER_NAME_BYTES: usize = 64;
pub(crate) const MAX_HEADER_VALUE_BYTES: usize = 1_024;
pub(crate) const MAX_HEADER_COUNT: usize = 32;
pub(crate) const MAX_HEADER_TOTAL_BYTES: usize = 8 * 1_024;
pub(crate) const MAX_BATCH_SIZE: usize = 100;
pub(crate) const MAX_LEASE_MILLIS: u64 = 60 * 60 * 1_000;
pub(crate) const MIN_LEASE_MILLIS: u64 = 1_000;
pub(crate) const MAX_RETRY_MILLIS: u64 = 7 * 24 * 60 * 60 * 1_000;
pub(crate) const MAX_DEAD_LETTER_QUERY: usize = 100;
pub(crate) const MAX_PURGE_BATCH: usize = 1_000;

pub(crate) fn validate_route_identifier(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<()> {
    if value.is_empty() || value.len() > max {
        return Err(MessagingError::Invalid {
            field,
            reason: "length is outside the supported range",
        });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
    {
        return Err(MessagingError::Invalid {
            field,
            reason: "only ASCII letters, digits, '.', '-', '_' and ':' are allowed",
        });
    }
    Ok(())
}

pub(crate) fn validate_idempotency_key(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_IDEMPOTENCY_BYTES {
        return Err(MessagingError::Invalid {
            field: "idempotency key",
            reason: "length is outside the supported range",
        });
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':' | b'/')
    }) {
        return Err(MessagingError::Invalid {
            field: "idempotency key",
            reason: "only bounded ASCII correlation characters are allowed",
        });
    }
    Ok(())
}

pub(crate) fn validate_content_type(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_CONTENT_TYPE_BYTES || !value.is_ascii() {
        return Err(MessagingError::Invalid {
            field: "content type",
            reason: "must be bounded ASCII",
        });
    }
    let mut pieces = value.split('/');
    let Some(primary) = pieces.next() else {
        return Err(MessagingError::Invalid {
            field: "content type",
            reason: "must contain one type/subtype separator",
        });
    };
    let Some(subtype) = pieces.next() else {
        return Err(MessagingError::Invalid {
            field: "content type",
            reason: "must contain one type/subtype separator",
        });
    };
    if pieces.next().is_some()
        || primary.is_empty()
        || subtype.is_empty()
        || !primary.bytes().all(is_mime_token)
        || !subtype.bytes().all(is_mime_token)
    {
        return Err(MessagingError::Invalid {
            field: "content type",
            reason: "must be one parameter-free MIME type/subtype",
        });
    }
    Ok(())
}

fn is_mime_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

pub(crate) fn validate_failure_code(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_FAILURE_CODE_BYTES {
        return Err(MessagingError::Invalid {
            field: "failure code",
            reason: "length is outside the supported range",
        });
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
    }) {
        return Err(MessagingError::Invalid {
            field: "failure code",
            reason: "only lowercase ASCII code characters are allowed",
        });
    }
    Ok(())
}

pub(crate) fn validate_header_name(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_HEADER_NAME_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
    {
        return Err(MessagingError::Invalid {
            field: "message header name",
            reason: "must be a bounded lowercase ASCII identifier",
        });
    }
    Ok(())
}

pub(crate) fn validate_header_value(value: &str) -> Result<()> {
    if value.len() > MAX_HEADER_VALUE_BYTES || value.chars().any(char::is_control) {
        return Err(MessagingError::Invalid {
            field: "message header value",
            reason: "must be bounded text without control characters",
        });
    }
    Ok(())
}
