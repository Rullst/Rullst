//! Opt-in fail-closed attachment inspection before transport dispatch.

use crate::drivers::MailDriver;
use crate::security::{redact_email_secrets, scan_content_security};
use crate::{Attachment, DeliveryPipeline, MailError, Message};
use async_trait::async_trait;
use sniff::{Kind, Markup, contains_ascii_case_insensitive};

mod sniff;

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
/// Checks are chosen from the case-insensitive declared type, the filename
/// extension and the content signature together, never from the declared type
/// alone. Every policy rejects executable magic and executable or script-host
/// extensions, SVG, active PDF content wherever a `%PDF-` header appears, and a
/// declared type that disagrees with a known extension or signature. The strict
/// policy also rejects HTML extensions, HTML/script markup or script URIs,
/// unknown extensions, declared types other than the inspected ones and
/// `application/octet-stream`, and opaque formats.
///
/// This is not antivirus, sandbox execution, recursive archive inspection or a
/// substitute for an independently operated content-disarm/scanning service.
/// PDF names written with `#xx` escapes or hidden in compressed streams are
/// not decoded.
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
        let content = attachment.content.as_slice();
        let strict = self.opaque_policy == OpaqueAttachmentPolicy::Reject;
        if executable_magic(content) {
            return Err(AttachmentInspectionError::Rejected("executable_content"));
        }
        // Recipients open files by extension, so the name is inspected too.
        let extension = sniff::extension(&attachment.filename);
        let extension = extension.as_deref();
        if extension.is_some_and(sniff::is_executable_extension) {
            return Err(AttachmentInspectionError::Rejected("executable_content"));
        }
        let declared = Kind::from_mime(&attachment.mime_type);
        let named = extension.and_then(Kind::from_extension);
        if declared
            .zip(named)
            .is_some_and(|(left, right)| left != right)
        {
            return Err(AttachmentInspectionError::Rejected("type_mismatch"));
        }
        let kind = declared.or(named);
        match kind {
            Some(Kind::Text) => inspect_text(content)?,
            Some(Kind::Pdf) => inspect_pdf(content)?,
            Some(Kind::Png) => require_prefix(content, b"\x89PNG\r\n\x1a\n")?,
            Some(Kind::Jpeg) => require_prefix(content, b"\xff\xd8\xff")?,
            Some(Kind::Gif) => {
                if !content.starts_with(b"GIF87a") && !content.starts_with(b"GIF89a") {
                    return Err(AttachmentInspectionError::Rejected("type_mismatch"));
                }
            }
            Some(Kind::Svg) => {
                return Err(AttachmentInspectionError::Rejected("active_svg_content"));
            }
            Some(Kind::Zip) => require_zip_signature(content)?,
            None => {}
        }
        // Content is inspected by what it is, whatever it claims to be.
        if sniff::looks_like_pdf(content) {
            reject_active_pdf(content)?;
        }
        let markup = sniff::markup(content);
        if markup == Some(Markup::Svg) {
            return Err(AttachmentInspectionError::Rejected("active_svg_content"));
        }
        if strict
            && (markup == Some(Markup::Active) || extension.is_some_and(sniff::is_html_extension))
        {
            return Err(AttachmentInspectionError::Rejected("active_markup_content"));
        }
        // A client may render by the declared type (e.g. `text/html`) whatever
        // the filename says; only the generic binary type makes no such claim.
        let unknown_declared = declared.is_none()
            && !attachment
                .mime_type
                .eq_ignore_ascii_case("application/octet-stream");
        match kind {
            // The strict policy cannot vouch for a type or extension it does not know.
            Some(_) if strict && (unknown_declared || (extension.is_some() && named.is_none())) => {
                self.opaque_result()
            }
            Some(Kind::Zip) | None => self.opaque_result(),
            Some(_) => Ok(()),
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
    async fn send_with_delivery_id(
        &self,
        message: &Message,
        delivery_id: &str,
    ) -> Result<(), MailError> {
        crate::drivers::traits::validate_delivery_id(delivery_id)?;
        let prepared = DeliveryPipeline::prepare(message)?;
        self.inspect_all(prepared.message()).await?;
        self.driver
            .send_with_delivery_id(prepared.message(), delivery_id)
            .await
    }

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
    reject_active_pdf(content)
}

/// Rejects PDF names that run code or carry active content: JavaScript and
/// launch actions, embedded files, XFA forms (whose XML carries JavaScript or
/// FormCalc scripts), rich media, embedded go-to and data-import actions.
fn reject_active_pdf(content: &[u8]) -> Result<(), AttachmentInspectionError> {
    for token in [
        b"/JavaScript".as_slice(),
        b"/JS",
        b"/Launch",
        b"/EmbeddedFile",
        b"/XFA",
        b"/RichMedia",
        b"/GoToE",
        b"/ImportData",
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

#[cfg(test)]
mod tests;
