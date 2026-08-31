// src/attachment.rs — Bounded owned email attachments and inline CID assets.

use crate::error::MailError;
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::path::Path;

/// Maximum attachments accepted by the shared delivery pipeline.
pub const MAX_ATTACHMENT_COUNT: usize = 32;
/// Maximum bytes accepted for one attachment before transport encoding.
pub const MAX_ATTACHMENT_BYTES: usize = 20 * 1024 * 1024;
/// Maximum aggregate attachment bytes accepted before transport encoding.
pub const MAX_TOTAL_ATTACHMENT_BYTES: usize = 25 * 1024 * 1024;
/// Maximum UTF-8 filename length in bytes.
pub const MAX_ATTACHMENT_FILENAME_BYTES: usize = 255;
/// Maximum parameter-free MIME type length in bytes.
pub const MAX_ATTACHMENT_MIME_BYTES: usize = 127;
/// Maximum Content-ID length in bytes, without angle brackets.
pub const MAX_ATTACHMENT_CID_BYTES: usize = 128;

/// Represents an email attachment or inline asset (e.g. image with Content-ID).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    /// Name of the attached file (e.g. `"invoice.pdf"`).
    pub filename: String,
    /// Raw byte payload of the attachment.
    pub content: Vec<u8>,
    /// MIME media type (e.g. `"application/pdf"`, `"image/png"`).
    pub mime_type: String,
    /// Optional Content-ID (CID) for inline HTML referencing (e.g. `<img src="cid:logo">`).
    pub cid: Option<String>,
}

impl Attachment {
    /// Creates a new `Attachment` from in-memory byte contents.
    pub fn new(
        filename: impl Into<String>,
        content: impl Into<Vec<u8>>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            filename: filename.into(),
            content: content.into(),
            mime_type: mime_type.into(),
            cid: None,
        }
    }

    /// Creates an inline `Attachment` with a designated Content-ID (CID).
    pub fn inline(
        cid: impl Into<String>,
        filename: impl Into<String>,
        content: impl Into<Vec<u8>>,
        mime_type: impl Into<String>,
    ) -> Self {
        let cid_str = cid.into();
        Self {
            filename: filename.into(),
            content: content.into(),
            mime_type: mime_type.into(),
            cid: Some(cid_str),
        }
    }

    /// Sets or overrides the Content-ID for inline HTML embedding.
    pub fn with_cid(mut self, cid: impl Into<String>) -> Self {
        self.cid = Some(cid.into());
        self
    }

    /// Reads an attachment directly from a file path on disk.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let path = path.as_ref();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment.bin")
            .to_string();

        let content = std::fs::read(path)?;
        let mime_type = match path.extension().and_then(|e| e.to_str()) {
            Some("pdf") => "application/pdf",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            Some("csv") => "text/csv",
            Some("json") => "application/json",
            Some("xml") => "application/xml",
            Some("zip") => "application/zip",
            Some("txt") => "text/plain",
            _ => "application/octet-stream",
        }
        .to_string();

        Ok(Self::new(filename, content, mime_type))
    }

    /// Encodes the binary content to a newly allocated Base64 string for REST payloads.
    pub fn to_base64(&self) -> String {
        BASE64_STANDARD.encode(&self.content)
    }

    /// Returns `true` if this attachment is marked as an inline asset with a CID.
    pub fn is_inline(&self) -> bool {
        self.cid.is_some()
    }
}

impl fmt::Debug for Attachment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Attachment")
            .field("filename", &self.filename)
            .field("content_bytes", &self.content.len())
            .field("mime_type", &self.mime_type)
            .field("cid", &self.cid)
            .finish()
    }
}

pub(crate) fn validate_attachment_set(
    attachments: &[Attachment],
    html: Option<&str>,
) -> Result<(), MailError> {
    if attachments.len() > MAX_ATTACHMENT_COUNT {
        return Err(validation_error(format!(
            "a message may contain at most {MAX_ATTACHMENT_COUNT} attachments"
        )));
    }

    let mut total_bytes = 0usize;
    let mut content_ids = HashSet::new();
    for attachment in attachments {
        validate_attachment(attachment)?;
        total_bytes = total_bytes
            .checked_add(attachment.content.len())
            .ok_or_else(|| validation_error("attachment byte accounting overflow"))?;
        if total_bytes > MAX_TOTAL_ATTACHMENT_BYTES {
            return Err(validation_error(format!(
                "aggregate attachment content must not exceed {MAX_TOTAL_ATTACHMENT_BYTES} bytes"
            )));
        }

        if let Some(cid) = attachment.cid.as_deref() {
            if !content_ids.insert(cid) {
                return Err(validation_error(
                    "attachment Content-ID values must be unique",
                ));
            }
            let Some(html) = html else {
                return Err(validation_error(
                    "inline CID attachments require an HTML body",
                ));
            };
            if !html_references_cid(html, cid) {
                return Err(validation_error(
                    "every inline attachment Content-ID must be referenced by the HTML body",
                ));
            }
        }
    }
    Ok(())
}

fn validate_attachment(attachment: &Attachment) -> Result<(), MailError> {
    if attachment.filename.trim().is_empty()
        || attachment.filename.len() > MAX_ATTACHMENT_FILENAME_BYTES
        || attachment.filename == "."
        || attachment.filename == ".."
        || attachment.filename.contains(['/', '\\'])
        || attachment.filename.chars().any(char::is_control)
    {
        return Err(validation_error(
            "attachment filename must be a bounded basename without control characters",
        ));
    }
    validate_mime_type(&attachment.mime_type)?;
    if attachment.content.len() > MAX_ATTACHMENT_BYTES {
        return Err(validation_error(format!(
            "one attachment must not exceed {MAX_ATTACHMENT_BYTES} bytes"
        )));
    }
    if let Some(cid) = attachment.cid.as_deref()
        && (cid.is_empty() || cid.len() > MAX_ATTACHMENT_CID_BYTES || !cid.bytes().all(is_cid_byte))
    {
        return Err(validation_error(
            "attachment Content-ID must be bounded ASCII without angle brackets",
        ));
    }
    Ok(())
}

fn validate_mime_type(value: &str) -> Result<(), MailError> {
    if value.is_empty() || value.len() > MAX_ATTACHMENT_MIME_BYTES || !value.is_ascii() {
        return Err(validation_error(
            "attachment MIME type must be bounded ASCII",
        ));
    }
    let mut pieces = value.split('/');
    let primary = pieces.next();
    let subtype = pieces.next();
    if pieces.next().is_some()
        || primary.is_none_or(str::is_empty)
        || subtype.is_none_or(str::is_empty)
        || !primary.is_some_and(|item| item.bytes().all(is_mime_token))
        || !subtype.is_some_and(|item| item.bytes().all(is_mime_token))
    {
        return Err(validation_error(
            "attachment MIME type must be one parameter-free type/subtype",
        ));
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

fn html_references_cid(html: &str, cid: &str) -> bool {
    let needle = format!("cid:{cid}");
    html.match_indices(&needle).any(|(start, _)| {
        let bytes = html.as_bytes();
        let before = start.checked_sub(1).and_then(|index| bytes.get(index));
        let after = bytes.get(start + needle.len());
        before.is_none_or(|byte| !is_cid_byte(*byte))
            && after.is_none_or(|byte| !is_cid_byte(*byte))
    })
}

fn is_cid_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':' | b'@')
}

fn validation_error(message: impl Into<String>) -> MailError {
    MailError::ValidationError(message.into())
}
