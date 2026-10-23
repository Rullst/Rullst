//! Bounded streaming `multipart/form-data` extractor.
//!
//! [`Multipart`] takes the request body; [`Multipart::read`] parses it under
//! an explicit [`MultipartPolicy`] into an [`UploadForm`]. Text fields are
//! kept in memory; each file streams to memory while it stays within
//! [`MultipartPolicy::memory_threshold`] and then to a temporary file, which
//! is removed when its [`UploadedFile`] is dropped unless
//! [`UploadedFile::persist_to`] moved it.

use super::UploadKind;
use axum::{
    body::{Body, Bytes},
    extract::{FromRequest, Request},
    http::header,
};
use http_body::Body as HttpBody;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

mod error;
mod file;
mod headers;
mod parser;
mod sniff;
mod sweep;

pub use error::MultipartError;
pub use file::UploadedFile;

use crate::security::MultipartFieldInspection;
use file::FileSink;
use parser::{Event, Parser};

/// Limits and content checks for [`Multipart::read`].
///
/// [`Default`] is a safe starting point: 10 MiB per file, 16 MiB per body, 4
/// files, 32 text fields of at most 64 KiB, field names of at most 128
/// bytes, files kept in memory up to 32 KiB, temporary files in
/// [`std::env::temp_dir`] and 30 seconds of body inactivity. Exceeding a
/// limit stops reading at once: the rest of the body is never read.
///
/// The declared `Content-Type` of a file part is never trusted. Content
/// checks look at the bytes: [`Self::allowed_kinds`] admits only the
/// [`UploadKind`]s that [`super::UploadPolicy`] recognizes, and
/// [`Self::expect_signature`] only files that start with one of the given
/// byte strings. A file that fails them ends the read with
/// [`MultipartError::ContentDenied`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MultipartPolicy {
    max_file_bytes: u64,
    max_total_bytes: u64,
    max_files: usize,
    max_fields: usize,
    max_field_bytes: usize,
    max_field_name_bytes: usize,
    memory_threshold: usize,
    temp_dir: Option<PathBuf>,
    allowed_kinds: Vec<UploadKind>,
    signatures: Vec<Vec<u8>>,
    idle_timeout: Duration,
}

impl Default for MultipartPolicy {
    fn default() -> Self {
        Self {
            max_file_bytes: Self::DEFAULT_MAX_FILE_BYTES,
            max_total_bytes: Self::DEFAULT_MAX_TOTAL_BYTES,
            max_files: Self::DEFAULT_MAX_FILES,
            max_fields: Self::DEFAULT_MAX_FIELDS,
            max_field_bytes: Self::DEFAULT_MAX_FIELD_BYTES,
            max_field_name_bytes: Self::DEFAULT_MAX_FIELD_NAME_BYTES,
            memory_threshold: Self::DEFAULT_MEMORY_THRESHOLD,
            temp_dir: None,
            allowed_kinds: Vec::new(),
            signatures: Vec::new(),
            idle_timeout: Self::DEFAULT_IDLE_TIMEOUT,
        }
    }
}

impl MultipartPolicy {
    /// Default largest file: 10 MiB.
    pub const DEFAULT_MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
    /// Default largest body, part headers and boundaries included: 16 MiB.
    pub const DEFAULT_MAX_TOTAL_BYTES: u64 = 16 * 1024 * 1024;
    /// Default largest number of files.
    pub const DEFAULT_MAX_FILES: usize = 4;
    /// Default largest number of text fields, `_token` included.
    pub const DEFAULT_MAX_FIELDS: usize = 32;
    /// Default largest text field value: 64 KiB.
    pub const DEFAULT_MAX_FIELD_BYTES: usize = 64 * 1024;
    /// Default longest field name: 128 bytes.
    pub const DEFAULT_MAX_FIELD_NAME_BYTES: usize = 128;
    /// Default largest file kept in memory: 32 KiB.
    pub const DEFAULT_MEMORY_THRESHOLD: usize = 32 * 1024;
    /// Default longest wait for the next piece of the body: 30 seconds.
    pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
    /// Largest accepted [`Self::memory_threshold`]: 1 MiB.
    pub const MAX_MEMORY_THRESHOLD: usize = 1024 * 1024;
    /// Longest accepted [`Self::expect_signature`]: 64 bytes.
    pub const MAX_SIGNATURE_BYTES: usize = 64;

    /// The default policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Largest accepted file, in bytes ([`MultipartError::FileTooLarge`]).
    pub fn max_file_bytes(mut self, bytes: u64) -> Self {
        self.max_file_bytes = bytes;
        self
    }

    /// Largest accepted body, part headers and boundaries included
    /// ([`MultipartError::TotalTooLarge`]). A larger `Content-Length` is
    /// refused before any byte is read.
    pub fn max_total_bytes(mut self, bytes: u64) -> Self {
        self.max_total_bytes = bytes;
        self
    }

    /// Largest number of files ([`MultipartError::TooManyFiles`]). An empty
    /// file input (a part with `filename=""` and no content, which browsers
    /// send for a file input left empty) is skipped and not counted.
    pub fn max_files(mut self, files: usize) -> Self {
        self.max_files = files;
        self
    }

    /// Largest number of text fields ([`MultipartError::TooManyFields`]).
    pub fn max_fields(mut self, fields: usize) -> Self {
        self.max_fields = fields;
        self
    }

    /// Largest text field value, in bytes ([`MultipartError::FieldTooLarge`]).
    pub fn max_field_bytes(mut self, bytes: usize) -> Self {
        self.max_field_bytes = bytes;
        self
    }

    /// Longest field name, in bytes ([`MultipartError::FieldNameTooLong`]).
    pub fn max_field_name_bytes(mut self, bytes: usize) -> Self {
        self.max_field_name_bytes = bytes;
        self
    }

    /// Largest file kept in memory; a larger one streams to a temporary
    /// file. At most [`Self::MAX_MEMORY_THRESHOLD`]; `0` writes every
    /// non-empty file to disk.
    pub fn memory_threshold(mut self, bytes: usize) -> Self {
        self.memory_threshold = bytes;
        self
    }

    /// Directory for temporary files, instead of [`std::env::temp_dir`].
    /// Each file is created there with a random name, `create_new` and, on
    /// Unix, mode `0600`. Choose a directory on the same file system as the
    /// [`UploadedFile::persist_to`] target, so persisting is a link, not a
    /// copy.
    ///
    /// Before the first temporary file a process writes to a directory, and
    /// again after each further 24 hours, the leftovers of a crashed process
    /// are removed there: regular files named `.rullst-upload-<32 lowercase
    /// hex digits>.part` last modified at least 24 hours ago, at most 1,000
    /// per sweep. Symbolic links, directories and other names are kept, and a
    /// failure is logged as a warning without failing the upload.
    pub fn temp_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.temp_dir = Some(dir.into());
        self
    }

    /// Admits only files whose content is one of `kinds`, recognized as
    /// [`super::UploadPolicy`] does.
    pub fn allowed_kinds(mut self, kinds: impl IntoIterator<Item = UploadKind>) -> Self {
        self.allowed_kinds = kinds.into_iter().collect();
        self.allowed_kinds.sort_unstable();
        self.allowed_kinds.dedup();
        self
    }

    /// Admits only files that start with `signature` or with another
    /// signature given to this method, such as `b"%PDF-"`. Signatures are
    /// 1 to [`Self::MAX_SIGNATURE_BYTES`] bytes long.
    pub fn expect_signature(mut self, signature: impl Into<Vec<u8>>) -> Self {
        self.signatures.push(signature.into());
        self
    }

    /// Longest wait for the next piece of the body
    /// ([`MultipartError::Timeout`]).
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    fn validate(&self) -> Result<(), MultipartError> {
        if self.memory_threshold > Self::MAX_MEMORY_THRESHOLD {
            return Err(MultipartError::InvalidPolicy(
                "memory_threshold exceeds 1 MiB",
            ));
        }
        if self
            .signatures
            .iter()
            .any(|signature| !(1..=Self::MAX_SIGNATURE_BYTES).contains(&signature.len()))
        {
            return Err(MultipartError::InvalidPolicy(
                "a signature must be 1 to 64 bytes long",
            ));
        }
        Ok(())
    }

    fn temp_directory(&self) -> PathBuf {
        self.temp_dir.clone().unwrap_or_else(std::env::temp_dir)
    }
}

/// A `multipart/form-data` request body, read with [`Self::read`].
///
/// Extracting it only checks the `Content-Type` (a missing or malformed
/// boundary is [`MultipartError::NotMultipart`]); it takes the body as it
/// arrives and ignores axum's `DefaultBodyLimit`:
/// [`MultipartPolicy::max_total_bytes`] bounds it instead.
///
/// # CSRF
///
/// The CSRF middleware reads a multipart body only until its first `_token`
/// field ends, within the first 64 KiB, so render `_token` as the form's
/// first field, before any file input, or send the `X-CSRF-Token` header.
/// Otherwise staging and production answer `403` before the handler runs,
/// and in every other environment [`Self::read`] fails with
/// [`MultipartError::CsrfTokenTooLate`] (also `403`, with the same message),
/// so the mistake shows up in development.
///
/// # WAF
///
/// The WAF does not read multipart bodies. Where it runs (staging and
/// production, or report-only in development) and the path is not in
/// `waf_body_exempt_paths`, [`Self::read`] applies its signatures to the
/// name and value of each text field: production refuses a match with
/// [`MultipartError::WafRefused`] and development logs it. File contents and
/// file names are not inspected.
///
/// ```rust,no_run
/// use rullst_core::uploads::{Multipart, MultipartError, MultipartPolicy, UploadKind};
///
/// async fn upload(multipart: Multipart) -> Result<&'static str, MultipartError> {
///     let policy = MultipartPolicy::default()
///         .max_file_bytes(50 * 1024 * 1024)
///         .max_total_bytes(51 * 1024 * 1024)
///         .max_files(1)
///         .allowed_kinds([UploadKind::Pdf]);
///     let mut form = multipart.read(&policy).await?;
///     let _title = form.field("title").unwrap_or_default().to_owned();
///     if let Some(document) = form.take_file("document") {
///         let name = format!("storage/documents/{}.pdf", document.sha256_hex());
///         document.persist_to(name).await?;
///     }
///     Ok("stored")
/// }
/// ```
#[derive(Debug)]
pub struct Multipart {
    boundary: String,
    body: Body,
    request: RequestFacts,
}

/// What [`Multipart::read`] needs to know about the request besides its
/// body. Kept apart from the body, which is not `Sync`, so the read future
/// stays `Send`.
#[derive(Debug)]
struct RequestFacts {
    declared_length: Option<u64>,
    has_csrf_header: bool,
    inspection: Option<MultipartFieldInspection>,
    path: String,
}

impl<S: Send + Sync> FromRequest<S> for Multipart {
    type Rejection = MultipartError;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers();
        let boundary = headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(crate::security::form_data_boundary)
            .ok_or(MultipartError::NotMultipart)?
            .to_owned();
        let declared_length = headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let has_csrf_header = headers.contains_key("x-csrf-token");
        let inspection = req.extensions().get::<MultipartFieldInspection>().copied();
        let path = req.uri().path().to_owned();
        Ok(Self {
            boundary,
            body: req.into_body(),
            request: RequestFacts {
                declared_length,
                has_csrf_header,
                inspection,
                path,
            },
        })
    }
}

/// The part being read.
enum Current {
    /// A text field and the value bytes read so far.
    Field { name: String, value: Vec<u8> },
    /// A file part.
    File(Box<FileSink>),
}

/// Counters of one [`Multipart::read`].
#[derive(Default)]
struct Tally {
    received: u64,
    files: usize,
    fields: usize,
    token_seen: bool,
}

impl Multipart {
    /// Reads the whole form under `policy`.
    ///
    /// Fails at the first limit, malformed part or denied content; files
    /// already read are dropped with their temporary files. The `_token`
    /// field stays among the text fields.
    pub async fn read(self, policy: &MultipartPolicy) -> Result<UploadForm, MultipartError> {
        policy.validate()?;
        let Self {
            boundary,
            mut body,
            request,
        } = self;
        if request
            .declared_length
            .is_some_and(|length| length > policy.max_total_bytes)
        {
            return Err(MultipartError::TotalTooLarge);
        }
        let mut parser = Parser::new(&boundary);
        let mut form = UploadForm::default();
        let mut tally = Tally::default();
        let mut current = None;
        loop {
            match parser.next_event()? {
                Event::NeedMore => match next_chunk(&mut body, policy.idle_timeout).await? {
                    Some(chunk) => {
                        tally.received = tally.received.saturating_add(chunk.len() as u64);
                        if tally.received > policy.max_total_bytes {
                            return Err(MultipartError::TotalTooLarge);
                        }
                        parser.push(&chunk);
                    }
                    None => parser.finish(),
                },
                Event::PartStart(raw) => {
                    current = Some(start_part(&raw, policy, &mut tally)?);
                }
                Event::Data(data) => {
                    if let Some(part) = current.as_mut() {
                        append(part, &data, policy, &mut tally).await?;
                    }
                }
                Event::PartEnd(data) => {
                    if let Some(mut part) = current.take() {
                        append(&mut part, &data, policy, &mut tally).await?;
                        let consumed = parser.consumed();
                        end_part(&request, part, consumed, policy, &mut tally, &mut form).await?;
                    }
                }
                Event::End => return Ok(form),
            }
        }
    }
}

/// The next data chunk of `body`, `None` at its end.
async fn next_chunk(body: &mut Body, idle: Duration) -> Result<Option<Bytes>, MultipartError> {
    loop {
        let frame = tokio::time::timeout(
            idle,
            std::future::poll_fn(|cx| Pin::new(&mut *body).poll_frame(cx)),
        )
        .await
        .map_err(|_| MultipartError::Timeout)?;
        match frame {
            None => return Ok(None),
            Some(Err(_)) => return Err(MultipartError::BodyRead),
            // A trailers frame carries no data; the body ends after it.
            Some(Ok(frame)) => {
                if let Ok(data) = frame.into_data() {
                    return Ok(Some(data));
                }
            }
        }
    }
}

async fn end_part(
    request: &RequestFacts,
    part: Current,
    consumed: u64,
    policy: &MultipartPolicy,
    tally: &mut Tally,
    form: &mut UploadForm,
) -> Result<(), MultipartError> {
    match part {
        Current::Field { name, value } => {
            let value = String::from_utf8(value).map_err(|_| MultipartError::FieldNotUtf8)?;
            if request
                .inspection
                .is_some_and(|inspection| inspection.refuses(&request.path, &name, &value))
            {
                return Err(MultipartError::WafRefused);
            }
            if name == "_token" && !tally.token_seen {
                tally.token_seen = true;
                // The CSRF middleware sees this field only if the body up to
                // its closing boundary fits its bounded prefix.
                let readable = consumed <= crate::security::MAX_TOKEN_PREFIX_BYTES as u64;
                if !readable && !request.has_csrf_header {
                    return Err(MultipartError::CsrfTokenTooLate);
                }
            }
            form.fields.push((name, value));
        }
        Current::File(sink) => {
            if let Some(file) = sink.finish(policy).await? {
                form.files.push(file);
            }
        }
    }
    Ok(())
}

fn start_part(
    raw: &[u8],
    policy: &MultipartPolicy,
    tally: &mut Tally,
) -> Result<Current, MultipartError> {
    let part = headers::parse(raw, policy.max_field_name_bytes)?;
    match part.file {
        None => {
            tally.fields += 1;
            if tally.fields > policy.max_fields {
                return Err(MultipartError::TooManyFields);
            }
            Ok(Current::Field {
                name: part.name,
                value: Vec::new(),
            })
        }
        Some(meta) => {
            let sink = FileSink::new(part.name, meta, policy.temp_directory());
            if sink.counts_immediately() {
                count_file(policy, tally)?;
            }
            Ok(Current::File(Box::new(sink)))
        }
    }
}

fn count_file(policy: &MultipartPolicy, tally: &mut Tally) -> Result<(), MultipartError> {
    tally.files += 1;
    if tally.files > policy.max_files {
        return Err(MultipartError::TooManyFiles);
    }
    Ok(())
}

async fn append(
    part: &mut Current,
    data: &[u8],
    policy: &MultipartPolicy,
    tally: &mut Tally,
) -> Result<(), MultipartError> {
    if data.is_empty() {
        return Ok(());
    }
    match part {
        Current::Field { value, .. } => {
            if value.len() + data.len() > policy.max_field_bytes {
                return Err(MultipartError::FieldTooLarge);
            }
            value.extend_from_slice(data);
            Ok(())
        }
        Current::File(sink) => {
            // A part with `filename=""` counts as a file once it has content.
            if !sink.counts_immediately() && sink.is_empty() {
                count_file(policy, tally)?;
            }
            sink.write(data, policy).await
        }
    }
}

/// A form read by [`Multipart::read`]: text fields and files in body order.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct UploadForm {
    fields: Vec<(String, String)>,
    files: Vec<UploadedFile>,
}

impl UploadForm {
    /// The value of the first text field named `name`.
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.as_str())
    }

    /// Every text field as `(name, value)`, in body order.
    pub fn fields(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fields
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// Every file, in body order.
    pub fn files(&self) -> &[UploadedFile] {
        &self.files
    }

    /// The first file sent in the field named `name`.
    pub fn file(&self, name: &str) -> Option<&UploadedFile> {
        self.files.iter().find(|file| file.field_name() == name)
    }

    /// Removes and returns the first file sent in the field named `name`,
    /// for example to [`UploadedFile::persist_to`] it.
    pub fn take_file(&mut self, name: &str) -> Option<UploadedFile> {
        let index = self
            .files
            .iter()
            .position(|file| file.field_name() == name)?;
        Some(self.files.remove(index))
    }
}

#[cfg(test)]
mod read_tests;

#[cfg(test)]
mod router_tests;
