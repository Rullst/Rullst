// src/attachment.rs — Zero-copy email attachments & inline CID assets.

use base64::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents an email attachment or inline asset (e.g. image with Content-ID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Encodes the binary content to a Base64 string for REST API payloads.
    pub fn to_base64(&self) -> String {
        BASE64_STANDARD.encode(&self.content)
    }

    /// Returns `true` if this attachment is marked as an inline asset with a CID.
    pub fn is_inline(&self) -> bool {
        self.cid.is_some()
    }
}
