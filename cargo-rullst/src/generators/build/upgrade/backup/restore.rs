//! Bounded preflight and per-file replacement for the legacy upgrade backup.
//!
//! This is not a sandbox or an all-files atomic transaction. Other writers must
//! be stopped; path checks do not defend against a hostile concurrent rename.
use std::collections::BTreeSet;
use std::fs::{self, File, Metadata};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use tempfile::NamedTempFile;

pub(super) const MAX_INDEX_BYTES: u64 = 8 * 1024 * 1024;
pub(super) const MAX_ENTRIES: usize = 100_000;
pub(super) const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub(super) const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub(super) enum RestoreError {
    #[error("invalid upgrade backup: {0}")]
    Invalid(&'static str),
    #[error("upgrade backup I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error(
        "restore stopped after {completed} file operations: {reason}; backup retained, review the working tree before retrying"
    )]
    Apply { completed: usize, reason: String },
}

struct Entry {
    relative: PathBuf,
    snapshot: Option<(PathBuf, u64)>,
}

pub(super) fn restore_from(root: &Path, requested: &Path) -> Result<PathBuf, RestoreError> {
    let root = root.canonicalize()?;
    let requested = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };
    let relative = requested
        .strip_prefix(&root)
        .map_err(|_| RestoreError::Invalid("backup is outside the project"))?;
    validate_directory_creation(&root, relative)?;
    let backup = requested.canonicalize()?;
    let allowed = root.join("target/rullst-upgrades");
    if !backup.starts_with(&allowed) || backup == allowed {
        return Err(RestoreError::Invalid(
            "backup must be inside this project's target/rullst-upgrades directory",
        ));
    }
    validate_directory_creation(&backup, Path::new("files"))?;
    let files = backup.join("files");
    let index_meta = validate_file(&backup, Path::new("index.tsv"))?
        .ok_or(RestoreError::Invalid("missing index"))?;
    if index_meta.len() > MAX_INDEX_BYTES {
        return Err(RestoreError::Invalid("index exceeds 8 MiB"));
    }
    let mut index = String::new();
    File::open(backup.join("index.tsv"))?
        .take(MAX_INDEX_BYTES + 1)
        .read_to_string(&mut index)?;
    if index.len() as u64 > MAX_INDEX_BYTES {
        return Err(RestoreError::Invalid("index exceeds 8 MiB"));
    }
    let entries = plan(&root, &files, &index)?;

    // Stage every replacement before changing any original. A failed copy or
    // full disk during staging leaves originals intact. RAII removes staged files.
    let mut staged = Vec::with_capacity(entries.len());
    for entry in &entries {
        let replacement = match &entry.snapshot {
            Some((snapshot, expected_size)) => {
                let original = root.join(&entry.relative);
                let parent = original
                    .parent()
                    .ok_or(RestoreError::Invalid("missing parent"))?;
                let mut temporary = NamedTempFile::new_in(parent)?;
                let mut source = File::open(snapshot)?.take(expected_size + 1);
                if io::copy(&mut source, temporary.as_file_mut())? != *expected_size {
                    return Err(RestoreError::Invalid("snapshot changed while staging"));
                }
                temporary
                    .as_file()
                    .set_permissions(fs::metadata(snapshot)?.permissions())?;
                temporary.as_file().sync_all()?;
                Some(temporary)
            }
            None => None,
        };
        staged.push(replacement);
    }
    for (completed, (entry, replacement)) in entries.iter().zip(staged).enumerate() {
        apply(&root, entry, replacement).map_err(|error| RestoreError::Apply {
            completed,
            reason: error.to_string(),
        })?;
    }
    Ok(backup)
}

fn plan(root: &Path, files: &Path, index: &str) -> Result<Vec<Entry>, RestoreError> {
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    for line in index.lines() {
        if entries.len() == MAX_ENTRIES {
            return Err(RestoreError::Invalid("too many index entries"));
        }
        let (state, relative) = line
            .split_once('\t')
            .ok_or(RestoreError::Invalid("malformed index entry"))?;
        let relative = Path::new(relative);
        validate_relative_restore_path(relative)?;
        let key = relative.to_path_buf();
        #[cfg(windows)]
        let key = PathBuf::from(relative.to_string_lossy().to_lowercase());
        if !seen.insert(key) {
            return Err(RestoreError::Invalid("duplicate index entry"));
        }
        validate_file(root, relative)?;
        let snapshot = match state {
            "present" => {
                let metadata = validate_file(files, relative)?
                    .ok_or(RestoreError::Invalid("missing snapshot"))?;
                total = total
                    .checked_add(metadata.len())
                    .ok_or(RestoreError::Invalid("snapshot size overflow"))?;
                if metadata.len() > MAX_FILE_BYTES || total > MAX_TOTAL_BYTES {
                    return Err(RestoreError::Invalid(
                        "snapshot exceeds 64 MiB/file or 512 MiB/restore",
                    ));
                }
                Some((files.join(relative), metadata.len()))
            }
            "absent" if relative == Path::new("Cargo.lock") => None,
            "absent" => {
                return Err(RestoreError::Invalid(
                    "only a newly created root Cargo.lock may be removed",
                ));
            }
            _ => return Err(RestoreError::Invalid("unknown index entry state")),
        };
        entries.push(Entry {
            relative: relative.to_path_buf(),
            snapshot,
        });
    }
    if entries.is_empty() {
        return Err(RestoreError::Invalid("empty index"));
    }
    Ok(entries)
}

fn apply(
    root: &Path,
    entry: &Entry,
    replacement: Option<NamedTempFile>,
) -> Result<(), RestoreError> {
    let existing = validate_file(root, &entry.relative)?;
    let original = root.join(&entry.relative);
    if let Some(temporary) = replacement {
        // Replace a directory entry rather than truncating an inode: a hardlink
        // at the destination must not overwrite a different file outside it.
        temporary
            .persist(original)
            .map_err(|error| RestoreError::Io(error.error))?;
    } else if existing.is_some() {
        fs::remove_file(original)?;
    }
    Ok(())
}

pub(super) fn validate_relative_restore_path(path: &Path) -> Result<(), RestoreError> {
    let text = path
        .to_str()
        .ok_or(RestoreError::Invalid("non-UTF-8 index path"))?;
    #[cfg(not(windows))]
    if text.contains('\\') {
        return Err(RestoreError::Invalid("non-portable index path"));
    }
    if text.is_empty()
        || text.chars().any(|c| c.is_control() || c == ':')
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        || path.components().any(|component| {
            component.as_os_str().to_str().is_some_and(|name| {
                name.eq_ignore_ascii_case(".git") || name.eq_ignore_ascii_case("target")
            })
        })
    {
        return Err(RestoreError::Invalid("unsafe relative path"));
    }
    if path != Path::new("Cargo.lock")
        && path.file_name().and_then(|name| name.to_str()) != Some("Cargo.toml")
        && path.extension().and_then(|ext| ext.to_str()) != Some("rs")
    {
        return Err(RestoreError::Invalid(
            "file outside the upgrade snapshot contract",
        ));
    }
    Ok(())
}

pub(super) fn validate_file(
    root: &Path,
    relative: &Path,
) -> Result<Option<Metadata>, RestoreError> {
    let parent = relative
        .parent()
        .ok_or(RestoreError::Invalid("missing parent"))?;
    validate_directory_creation(root, parent)?;
    if !root.join(parent).is_dir() {
        return Err(RestoreError::Invalid("missing file parent"));
    }
    match fs::symlink_metadata(root.join(relative)) {
        Ok(metadata) if !is_link(&metadata) && metadata.is_file() => Ok(Some(metadata)),
        Ok(_) => Err(RestoreError::Invalid(
            "symlink, reparse point or non-regular file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn validate_directory_creation(
    root: &Path,
    relative: &Path,
) -> Result<(), RestoreError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(RestoreError::Invalid("unsafe directory path"));
        }
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if !is_link(&metadata) && metadata.is_dir() => (),
            Ok(_) => {
                return Err(RestoreError::Invalid(
                    "symlink, reparse point or non-directory parent",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn is_link(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

#[cfg(test)]
#[path = "restore_tests.rs"]
mod tests;
