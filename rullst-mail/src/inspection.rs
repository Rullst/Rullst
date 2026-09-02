//! Opt-in fail-closed attachment inspection before transport dispatch.

use crate::drivers::MailDriver;
use crate::security::{redact_email_secrets, scan_content_security};
use crate::{Attachment, DeliveryPipeline, MailError, Message};
use async_trait::async_trait;

/// Typed inspection failures which omit filenames and attachment bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AttachmentInspectionError {
    /// Content violates the configured policy.
    Rejected(&'static str),
    /// The scanner could not make an authoritative decision.
    Unavailable,
}

impl std::fmt::Display for AttachmentInspectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rejected(reason) => write!(formatter, "attachment rejected: {reason}"),
            Self::Unavailable => formatter.write_str("attachment inspection unavailable"),
        }
    }
}

impl std::error::Error for AttachmentInspectionError {}

/// Static-dispatch contract for local or external content scanners.
pub trait AttachmentInspector: Send + Sync {
    /// Inspects one already size/metadata-validated attachment.
    fn inspect(
        &self,
        attachment: &Attachment,
    ) -> impl std::future::Future<Output = Result<(), AttachmentInspectionError>> + Send;
}

/// How the bounded local inspector handles formats it cannot parse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OpaqueAttachmentPolicy {
    /// Reject every unsupported or archive format.
    Reject,
    /// Accept unsupported formats after executable-magic checks.
    Allow,
}

/// Bounded local type/signature, active-content, URL and secret heuristic.
///
/// This is not antivirus, sandbox execution, recursive archive inspection or a
/// substitute for an independently operated content-disarm/scanning service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalAttachmentInspector {
    opaque_policy: OpaqueAttachmentPolicy,
}

impl LocalAttachmentInspector {
    /// Rejects formats which the local inspector cannot inspect.
    pub const fn strict() -> Self {
        Self {
            opaque_policy: OpaqueAttachmentPolicy::Reject,
        }
    }

    /// Allows opaque formats after bounded executable-magic checks.
    pub const fn allowing_opaque() -> Self {
        Self {
            opaque_policy: OpaqueAttachmentPolicy::Allow,
        }
    }

    fn inspect_local(&self, attachment: &Attachment) -> Result<(), AttachmentInspectionError> {
        if executable_magic(&attachment.content) {
            return Err(AttachmentInspectionError::Rejected("executable_content"));
        }
        match attachment.mime_type.as_str() {
            "text/plain" | "text/csv" | "application/json" | "application/xml" => {
                inspect_text(&attachment.content)
            }
            "application/pdf" => inspect_pdf(&attachment.content),
            "image/png" => require_prefix(&attachment.content, b"\x89PNG\r\n\x1a\n"),
            "image/jpeg" => require_prefix(&attachment.content, b"\xff\xd8\xff"),
            "image/gif" => {
                if attachment.content.starts_with(b"GIF87a")
                    || attachment.content.starts_with(b"GIF89a")
                {
                    Ok(())
                } else {
                    Err(AttachmentInspectionError::Rejected("type_mismatch"))
                }
            }
            "image/svg+xml" => Err(AttachmentInspectionError::Rejected("active_svg_content")),
            "application/zip" => {
                require_zip_signature(&attachment.content)?;
                self.opaque_result()
            }
            "application/octet-stream" => self.opaque_result(),
            _ => self.opaque_result(),
        }
    }

    fn opaque_result(&self) -> Result<(), AttachmentInspectionError> {
        match self.opaque_policy {
            OpaqueAttachmentPolicy::Reject => {
                Err(AttachmentInspectionError::Rejected("opaque_content"))
            }
            OpaqueAttachmentPolicy::Allow => Ok(()),
        }
    }
}

impl AttachmentInspector for LocalAttachmentInspector {
    async fn inspect(&self, attachment: &Attachment) -> Result<(), AttachmentInspectionError> {
        self.inspect_local(attachment)
    }
}

/// Driver wrapper which completes every configured inspection before delivery.
pub struct AttachmentInspectionGuard<D, I> {
    driver: D,
    inspector: I,
}

impl<D, I> AttachmentInspectionGuard<D, I> {
    /// Wraps a driver and scanner using static dispatch.
    pub const fn new(driver: D, inspector: I) -> Self {
        Self { driver, inspector }
    }

    /// Returns the wrapped driver.
    pub const fn driver(&self) -> &D {
        &self.driver
    }

    /// Returns the configured inspector.
    pub const fn inspector(&self) -> &I {
        &self.inspector
    }

    async fn inspect_all(&self, message: &Message) -> Result<(), MailError>
    where
        I: AttachmentInspector,
    {
        for attachment in &message.attachments {
            self.inspector
                .inspect(attachment)
                .await
                .map_err(|error| match error {
                    AttachmentInspectionError::Rejected(reason) => {
                        MailError::AttachmentRejected { reason }
                    }
                    AttachmentInspectionError::Unavailable => {
                        MailError::AttachmentInspectionUnavailable
                    }
                })?;
        }
        Ok(())
    }
}

#[async_trait]
impl<D, I> MailDriver for AttachmentInspectionGuard<D, I>
where
    D: MailDriver,
    I: AttachmentInspector,
{
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare(message)?;
        self.inspect_all(prepared.message()).await?;
        self.driver.send(prepared.message()).await
    }

    async fn send_for_tenant(&self, tenant_id: &str, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare_for_tenant(tenant_id, message)?;
        self.inspect_all(prepared.message()).await?;
        self.driver
            .send_for_tenant(tenant_id, prepared.message())
            .await
    }
}

fn inspect_text(content: &[u8]) -> Result<(), AttachmentInspectionError> {
    let text = std::str::from_utf8(content)
        .map_err(|_| AttachmentInspectionError::Rejected("invalid_text_encoding"))?;
    if text.contains('\0') {
        return Err(AttachmentInspectionError::Rejected("binary_text_content"));
    }
    if redact_email_secrets(text) != text {
        return Err(AttachmentInspectionError::Rejected("secret_detected"));
    }
    scan_content_security(text)
        .map_err(|_| AttachmentInspectionError::Rejected("unsafe_link_content"))
}

fn inspect_pdf(content: &[u8]) -> Result<(), AttachmentInspectionError> {
    require_prefix(content, b"%PDF-")?;
    for token in [
        b"/JavaScript".as_slice(),
        b"/JS",
        b"/Launch",
        b"/EmbeddedFile",
    ] {
        if contains_ascii_case_insensitive(content, token) {
            return Err(AttachmentInspectionError::Rejected("active_pdf_content"));
        }
    }
    Ok(())
}

fn require_prefix(content: &[u8], prefix: &[u8]) -> Result<(), AttachmentInspectionError> {
    if content.starts_with(prefix) {
        Ok(())
    } else {
        Err(AttachmentInspectionError::Rejected("type_mismatch"))
    }
}

fn require_zip_signature(content: &[u8]) -> Result<(), AttachmentInspectionError> {
    if content.starts_with(b"PK\x03\x04")
        || content.starts_with(b"PK\x05\x06")
        || content.starts_with(b"PK\x07\x08")
    {
        Ok(())
    } else {
        Err(AttachmentInspectionError::Rejected("type_mismatch"))
    }
}

fn executable_magic(content: &[u8]) -> bool {
    content.starts_with(b"MZ")
        || content.starts_with(b"\x7fELF")
        || content.starts_with(&[0xfe, 0xed, 0xfa, 0xce])
        || content.starts_with(&[0xce, 0xfa, 0xed, 0xfe])
        || content.starts_with(&[0xfe, 0xed, 0xfa, 0xcf])
        || content.starts_with(&[0xcf, 0xfa, 0xed, 0xfe])
}

fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}

#[cfg(test)]
mod tests;
