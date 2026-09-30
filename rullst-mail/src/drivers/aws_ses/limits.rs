//! SES v2 field and encoded-size bounds shared by the native and proxy transports.

use crate::error::MailError;
use crate::message::Message;

pub(super) const MAX_SES_V2_MESSAGE_BYTES: usize = 40 * 1024 * 1024;
const MESSAGE_ENVELOPE_ALLOWANCE: usize = 4 * 1024;

/// Rejects SES field limits and an encoded estimate over 40 MiB before network I/O.
pub(super) fn validate_message_limits(message: &Message) -> Result<(), MailError> {
    let mut size = MESSAGE_ENVELOPE_ALLOWANCE;
    for value in [
        Some(&message.to),
        Some(&message.subject),
        message.from.as_ref(),
        message.body_html.as_ref(),
        message.body_text.as_ref(),
        message.unsubscribe_url.as_ref(),
        message.unsubscribe_email.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        add_size(&mut size, json_string_size(value)?)?;
    }

    for attachment in &message.attachments {
        if attachment.filename.is_empty() || attachment.filename.len() > 255 {
            return Err(MailError::ValidationError(
                "AWS SES attachment filename must contain 1-255 bytes".to_string(),
            ));
        }
        if attachment.mime_type.is_empty() || attachment.mime_type.len() > 78 {
            return Err(MailError::ValidationError(
                "AWS SES attachment MIME type must contain 1-78 bytes".to_string(),
            ));
        }
        if attachment
            .cid
            .as_ref()
            .is_some_and(|cid| cid.is_empty() || cid.len() > 78)
        {
            return Err(MailError::ValidationError(
                "AWS SES attachment Content-ID must contain 1-78 bytes".to_string(),
            ));
        }
        add_size(&mut size, json_string_size(&attachment.filename)?)?;
        add_size(&mut size, json_string_size(&attachment.mime_type)?)?;
        if let Some(cid) = &attachment.cid {
            add_size(&mut size, json_string_size(cid)?)?;
        }
        let encoded = attachment
            .content
            .len()
            .checked_add(2)
            .and_then(|length| length.checked_div(3))
            .and_then(|length| length.checked_mul(4))
            .ok_or_else(message_size_error)?;
        add_size(&mut size, encoded)?;
    }
    Ok(())
}

fn json_string_size(value: &str) -> Result<usize, MailError> {
    serde_json::to_vec(value)
        .map(|encoded| encoded.len())
        .map_err(|_| MailError::ValidationError("AWS SES message encoding failed".to_string()))
}

pub(super) fn add_size(total: &mut usize, additional: usize) -> Result<(), MailError> {
    *total = total
        .checked_add(additional)
        .ok_or_else(message_size_error)?;
    if *total > MAX_SES_V2_MESSAGE_BYTES {
        return Err(message_size_error());
    }
    Ok(())
}

pub(super) fn message_size_error() -> MailError {
    MailError::ValidationError(
        "AWS SES v2 message exceeds the 40 MiB encoded safety boundary".to_string(),
    )
}
