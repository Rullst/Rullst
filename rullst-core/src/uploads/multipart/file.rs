//! Storage of one uploaded file: memory first, then a temporary file.

use super::headers::FileMeta;
use super::sniff::Sniffer;
use super::{MultipartError, MultipartPolicy};
use crate::uploads::UploadKind;
use std::fmt;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncWriteExt;

fn storage(error: std::io::Error) -> MultipartError {
    MultipartError::Storage(error.kind())
}

/// A temporary file, removed when this value is dropped.
struct TempPath(PathBuf);

impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Opens a new file at `path` that no one else can read: `create_new`, and
/// mode `0600` on Unix.
async fn create_new(path: &Path) -> std::io::Result<tokio::fs::File> {
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    options.open(path).await
}

/// A file part while it is read.
pub(super) struct FileSink {
    field_name: String,
    meta: FileMeta,
    temp_dir: PathBuf,
    memory: Vec<u8>,
    // Declared before `temp`, so the handle closes before the file is removed.
    spill: Option<tokio::fs::File>,
    temp: Option<TempPath>,
    len: u64,
    digest: ring::digest::Context,
    sniffer: Sniffer,
}

impl FileSink {
    pub(super) fn new(field_name: String, meta: FileMeta, temp_dir: PathBuf) -> Self {
        Self {
            field_name,
            meta,
            temp_dir,
            memory: Vec::new(),
            spill: None,
            temp: None,
            len: 0,
            digest: ring::digest::Context::new(&ring::digest::SHA256),
            sniffer: Sniffer::default(),
        }
    }

    /// Whether the part counts against `max_files` before any content: all
    /// but `filename=""`, the empty file input of a browser form.
    pub(super) fn counts_immediately(&self) -> bool {
        !self.meta.empty_file_name
    }

    /// Whether no content has been written.
    pub(super) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Appends `data`, enforcing the size limit and content checks first.
    pub(super) async fn write(
        &mut self,
        data: &[u8],
        policy: &MultipartPolicy,
    ) -> Result<(), MultipartError> {
        let len = self.len.saturating_add(data.len() as u64);
        if len > policy.max_file_bytes {
            return Err(MultipartError::FileTooLarge);
        }
        self.len = len;
        self.digest.update(data);
        self.sniffer.update(data);
        if self.sniffer.admits(policy, false) == Some(false) {
            return Err(MultipartError::ContentDenied);
        }
        if self.temp.is_none() && self.memory.len() + data.len() <= policy.memory_threshold {
            self.memory.extend_from_slice(data);
            return Ok(());
        }
        if self.spill.is_none() {
            self.spill = Some(self.open_spill().await?);
        }
        match self.spill.as_mut() {
            Some(file) => file.write_all(data).await.map_err(storage),
            None => Err(MultipartError::Storage(ErrorKind::NotFound)),
        }
    }

    /// Creates the temporary file and moves the bytes kept in memory to it.
    async fn open_spill(&mut self) -> Result<tokio::fs::File, MultipartError> {
        let path = self.temp_dir.join(format!(
            ".rullst-upload-{}.part",
            uuid::Uuid::new_v4().simple()
        ));
        let mut file = create_new(&path).await.map_err(storage)?;
        // Owned from here on, so a failure below still removes the file.
        self.temp = Some(TempPath(path));
        file.write_all(&self.memory).await.map_err(storage)?;
        self.memory = Vec::new();
        Ok(file)
    }

    /// Completes the file: `None` for an empty file input, which is skipped.
    pub(super) async fn finish(
        mut self,
        policy: &MultipartPolicy,
    ) -> Result<Option<UploadedFile>, MultipartError> {
        if self.meta.empty_file_name && self.len == 0 {
            return Ok(None);
        }
        if self.sniffer.admits(policy, true) != Some(true) {
            return Err(MultipartError::ContentDenied);
        }
        let storage = match (self.spill.take(), self.temp.take()) {
            (Some(mut file), Some(temp)) => {
                file.flush().await.map_err(storage)?;
                file.sync_all().await.map_err(storage)?;
                drop(file);
                Storage::Disk(temp)
            }
            _ => Storage::Memory(std::mem::take(&mut self.memory)),
        };
        Ok(Some(UploadedFile {
            kind: self.sniffer.kind(),
            sha256_hex: crate::uploads::encode_hex(self.digest.finish().as_ref()),
            field_name: self.field_name,
            display_name: self.meta.display_name,
            declared_content_type: self.meta.content_type,
            len: self.len,
            storage,
        }))
    }
}

enum Storage {
    Memory(Vec<u8>),
    Disk(TempPath),
}

/// A file read by [`super::Multipart::read`].
///
/// Its content is in memory or in a temporary file that is removed when this
/// value is dropped; [`Self::persist_to`] keeps it. Nothing the client sent
/// about the file is trusted: the name is display metadata and the declared
/// type is informational; [`Self::kind`] comes from the bytes.
pub struct UploadedFile {
    field_name: String,
    display_name: Option<String>,
    declared_content_type: Option<String>,
    kind: Option<UploadKind>,
    len: u64,
    sha256_hex: String,
    storage: Storage,
}

impl fmt::Debug for UploadedFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UploadedFile")
            .field("field_name", &self.field_name)
            .field("display_name", &self.display_name)
            .field("declared_content_type", &self.declared_content_type)
            .field("kind", &self.kind)
            .field("len", &self.len)
            .field("in_memory", &self.bytes().is_some())
            .finish_non_exhaustive()
    }
}

impl UploadedFile {
    /// Name of the form field that sent the file.
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// The client's file name for display only, never as a path: its last
    /// `/` or `\` segment without control or invisible format characters,
    /// trimmed and cut to 255 bytes. `None` when nothing is left, or when
    /// the client sent only an RFC 5987 `filename*`. Escape it when
    /// rendering.
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// The `Content-Type` the client declared, if printable. Informational
    /// only: use [`Self::kind`] to decide what the file is.
    pub fn declared_content_type(&self) -> Option<&str> {
        self.declared_content_type.as_deref()
    }

    /// The kind recognized from the content, as [`crate::uploads::UploadPolicy`]
    /// recognizes it, or `None` for anything else.
    pub fn kind(&self) -> Option<UploadKind> {
        self.kind
    }

    /// Content length in bytes.
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Whether the content is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Lowercase SHA-256 digest of the content.
    pub fn sha256_hex(&self) -> &str {
        &self.sha256_hex
    }

    /// The content, when it stayed within the policy's memory threshold.
    pub fn bytes(&self) -> Option<&[u8]> {
        match &self.storage {
            Storage::Memory(bytes) => Some(bytes),
            Storage::Disk(_) => None,
        }
    }

    /// The temporary file holding the content, when it was streamed to
    /// disk. It is removed when this value is dropped.
    pub fn temp_path(&self) -> Option<&Path> {
        match &self.storage {
            Storage::Memory(_) => None,
            Storage::Disk(temp) => Some(&temp.0),
        }
    }

    /// Reads the whole content into memory.
    pub async fn read_to_vec(&self) -> Result<Vec<u8>, MultipartError> {
        match &self.storage {
            Storage::Memory(bytes) => Ok(bytes.clone()),
            Storage::Disk(temp) => tokio::fs::read(&temp.0).await.map_err(storage),
        }
    }

    /// Stores the content at `target`, a new file in an existing directory.
    ///
    /// Never overwrites: an existing `target` (a symbolic link included) is
    /// [`MultipartError::PersistTargetExists`]. A target without a file
    /// name, with a `..` component or outside an existing directory is
    /// [`MultipartError::InvalidPersistTarget`]. A temporary file is linked
    /// into place when it is on the same file system and copied otherwise;
    /// the new file has mode `0600` on Unix and is synced to disk. Build
    /// `target` from trusted values, never from [`Self::display_name`].
    pub async fn persist_to(self, target: impl AsRef<Path>) -> Result<(), MultipartError> {
        let target = target.as_ref();
        validate_target(target).await?;
        match &self.storage {
            Storage::Memory(bytes) => write_new(target, bytes).await,
            Storage::Disk(temp) => {
                match link_outcome(tokio::fs::hard_link(&temp.0, target).await) {
                    Linked::Done => Ok(()),
                    Linked::TargetExists => Err(MultipartError::PersistTargetExists),
                    Linked::CopyInstead => copy_new(&temp.0, target).await,
                }
            }
        }
    }
}

/// What the result of hard-linking a temporary file to its target means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Linked {
    /// The target now names the file.
    Done,
    /// The target exists; it is never replaced.
    TargetExists,
    /// No link is possible here (another file system, or one without hard
    /// links): copy instead.
    CopyInstead,
}

fn link_outcome(result: std::io::Result<()>) -> Linked {
    match result {
        Ok(()) => Linked::Done,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Linked::TargetExists,
        Err(_) => Linked::CopyInstead,
    }
}

async fn validate_target(target: &Path) -> Result<(), MultipartError> {
    if target.file_name().is_none()
        || target
            .components()
            .any(|component| component == Component::ParentDir)
    {
        return Err(MultipartError::InvalidPersistTarget);
    }
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    match tokio::fs::metadata(parent).await {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        _ => Err(MultipartError::InvalidPersistTarget),
    }
}

/// Creates `target` with `create_new`; an existing path is
/// [`MultipartError::PersistTargetExists`].
async fn create_target(target: &Path) -> Result<tokio::fs::File, MultipartError> {
    create_new(target).await.map_err(|error| {
        if error.kind() == ErrorKind::AlreadyExists {
            MultipartError::PersistTargetExists
        } else {
            storage(error)
        }
    })
}

/// Removes a target this module created when filling it failed.
fn keep_or_remove(
    file: tokio::fs::File,
    written: std::io::Result<()>,
    target: &Path,
) -> Result<(), MultipartError> {
    drop(file);
    written.map_err(|error| {
        let _ = std::fs::remove_file(target);
        storage(error)
    })
}

async fn write_new(target: &Path, bytes: &[u8]) -> Result<(), MultipartError> {
    let mut file = create_target(target).await?;
    let written = async {
        file.write_all(bytes).await?;
        file.flush().await?;
        file.sync_all().await
    }
    .await;
    keep_or_remove(file, written, target)
}

async fn copy_new(source: &Path, target: &Path) -> Result<(), MultipartError> {
    let mut file = create_target(target).await?;
    let written = async {
        let mut input = tokio::fs::File::open(source).await?;
        tokio::io::copy(&mut input, &mut file).await?;
        file.flush().await?;
        file.sync_all().await
    }
    .await;
    keep_or_remove(file, written, target)
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
