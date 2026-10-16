//! Typed failures of the multipart extractor and their HTTP responses.

use axum::{
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

/// Why a multipart form could not be read or stored.
///
/// As a response it is plain text with `Cache-Control: no-store`: `413` for
/// the size and count limits, `415` for a body that is not
/// `multipart/form-data` or a file whose content is denied, `408` when the
/// body stalls, `403` for the WAF and CSRF refusals, `400` for anything else
/// the client sent, and `500` for server-side failures. No message contains
/// client data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MultipartError {
    /// The `Content-Type` is not `multipart/form-data` with a valid boundary.
    #[error("expected a multipart/form-data body with a valid boundary")]
    NotMultipart,
    /// The body (or its declared `Content-Length`) exceeds `max_total_bytes`.
    #[error("the multipart body exceeds its size limit")]
    TotalTooLarge,
    /// A file exceeds `max_file_bytes`.
    #[error("an uploaded file exceeds its size limit")]
    FileTooLarge,
    /// The form has more than `max_files` files.
    #[error("the form has too many files")]
    TooManyFiles,
    /// The form has more than `max_fields` text fields.
    #[error("the form has too many fields")]
    TooManyFields,
    /// A text field value exceeds `max_field_bytes`.
    #[error("a form field exceeds its size limit")]
    FieldTooLarge,
    /// A field name exceeds `max_field_name_bytes`.
    #[error("a form field name is too long")]
    FieldNameTooLong,
    /// A part's header block exceeds 8 KiB.
    #[error("a multipart part has too many header bytes")]
    PartHeadersTooLarge,
    /// The body is not a well-formed form; the reason is fixed text.
    #[error("malformed multipart body: {0}")]
    Malformed(&'static str),
    /// A text field value is not UTF-8.
    #[error("a form field is not valid UTF-8")]
    FieldNotUtf8,
    /// A file's content fails the policy's content checks.
    #[error("an uploaded file's content is not allowed")]
    ContentDenied,
    /// No body bytes arrived within the policy's idle timeout.
    #[error("the multipart body arrived too slowly")]
    Timeout,
    /// The body failed while it was read, for example a dropped connection.
    #[error("the request body could not be read")]
    BodyRead,
    /// A text field carries a WAF injection signature (staging and
    /// production).
    #[error("a form field was refused by the WAF")]
    WafRefused,
    /// The form's first `_token` field ends after the first 64 KiB of the
    /// body, where the CSRF middleware cannot read it, and no
    /// `X-CSRF-Token` header was sent.
    #[error(
        "the multipart form sends its _token field after the first 64 KiB; send it before any file field"
    )]
    CsrfTokenTooLate,
    /// A temporary or persisted file could not be written or read.
    #[error("the upload could not be stored ({0})")]
    Storage(std::io::ErrorKind),
    /// The [`super::UploadedFile::persist_to`] target is not a new file name
    /// in an existing directory, or has a `..` component.
    #[error("the upload target is not a file path in an existing directory")]
    InvalidPersistTarget,
    /// The [`super::UploadedFile::persist_to`] target already exists.
    #[error("the upload target already exists")]
    PersistTargetExists,
    /// The [`super::MultipartPolicy`] is invalid; the reason is fixed text.
    #[error("invalid multipart policy: {0}")]
    InvalidPolicy(&'static str),
}

impl MultipartError {
    /// The HTTP status this failure is answered with.
    pub fn status(&self) -> StatusCode {
        match self {
            Self::NotMultipart | Self::ContentDenied => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::TotalTooLarge
            | Self::FileTooLarge
            | Self::TooManyFiles
            | Self::TooManyFields
            | Self::FieldTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::FieldNameTooLong
            | Self::PartHeadersTooLarge
            | Self::Malformed(_)
            | Self::FieldNotUtf8
            | Self::BodyRead => StatusCode::BAD_REQUEST,
            Self::Timeout => StatusCode::REQUEST_TIMEOUT,
            Self::WafRefused | Self::CsrfTokenTooLate => StatusCode::FORBIDDEN,
            Self::Storage(_)
            | Self::InvalidPersistTarget
            | Self::PersistTargetExists
            | Self::InvalidPolicy(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for MultipartError {
    fn into_response(self) -> Response {
        let mut response = match &self {
            Self::WafRefused => return crate::security::multipart_field_refusal(),
            Self::CsrfTokenTooLate => {
                crate::security::MULTIPART_TOKEN_ORDER_MESSAGE.into_response()
            }
            // Server-side details stay in logs, not in the response.
            Self::Storage(_)
            | Self::InvalidPersistTarget
            | Self::PersistTargetExists
            | Self::InvalidPolicy(_) => "the upload could not be processed".into_response(),
            _ => self.to_string().into_response(),
        };
        *response.status_mut() = self.status();
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response
    }
}
