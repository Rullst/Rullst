//! The content hashes of the files below one `static/` directory.

use super::names::{HASH_LEN, hashable, hashed_name, plain_segment, plain_url, split_hashed};
use std::collections::HashMap;
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};
use std::time::SystemTime;

/// Bounds of the start-up walk and of every hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Limits {
    /// Files recorded at most.
    pub files: usize,
    /// Larger files keep their plain URL.
    pub file_bytes: u64,
    /// Directory levels below `static/` walked at start-up.
    pub depth: usize,
}

impl Limits {
    pub(crate) const DEFAULT: Self = Self {
        files: 10_000,
        // 32 MiB.
        file_bytes: 33_554_432,
        depth: 16,
    };
}

/// One recorded file: its hash and the metadata it was computed from.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    hash: String,
    len: u64,
    modified: Option<SystemTime>,
}

impl Entry {
    fn describes(&self, metadata: &Metadata) -> bool {
        self.len == metadata.len() && self.modified == metadata.modified().ok()
    }
}

/// How the static mount answers a request below `/static/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Resolution {
    /// Not the hashed name of a recorded file: served as requested.
    PassThrough,
    /// The hash is the file's current one: serve this real path, immutable.
    Current(String),
    /// The file changed since that hash: redirect to this current URL.
    Outdated(String),
}

/// Content hashes of the files below one `static/` directory.
#[derive(Debug)]
pub(crate) struct Manifest {
    directory: PathBuf,
    /// Debug builds: [`Manifest::url`] follows edits and new files.
    follow_edits: bool,
    limits: Limits,
    entries: RwLock<HashMap<String, Entry>>,
}

impl Manifest {
    /// Hashes the files below `directory` within `limits`. Unreadable
    /// entries, symbolic links and names [`hashable`] refuses are skipped.
    pub(crate) fn build(directory: impl Into<PathBuf>, follow_edits: bool, limits: Limits) -> Self {
        let directory = directory.into();
        let mut entries = HashMap::new();
        collect(&directory, "", 0, limits, &mut entries);
        Self {
            directory,
            follow_edits,
            limits,
            entries: RwLock::new(entries),
        }
    }

    /// Recorded files (tests).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// The hashed URL of `relative`, or its plain URL.
    pub(crate) fn url(&self, relative: &str) -> String {
        match self.current_hash(relative, self.follow_edits) {
            Some(hash) => plain_url(&hashed_name(relative, &hash)),
            None => plain_url(relative),
        }
    }

    /// How to answer a request for `requested` (the path below `/static/`).
    /// Checks the file's metadata, hashing it again after a change.
    pub(crate) fn resolve(&self, requested: &str) -> Resolution {
        let Some((base, hash)) = split_hashed(requested) else {
            return Resolution::PassThrough;
        };
        // A real entry with that name is served as it is.
        if fs::symlink_metadata(self.directory.join(requested)).is_ok() {
            return Resolution::PassThrough;
        }
        match self.current_hash(&base, true) {
            Some(current) if current == hash => Resolution::Current(base),
            Some(current) => Resolution::Outdated(plain_url(&hashed_name(&base, &current))),
            None => Resolution::PassThrough,
        }
    }

    /// The hash of `relative`; with `verify`, after comparing the recorded
    /// metadata with the file's (and, with `follow_edits`, hashing a file
    /// not recorded yet).
    fn current_hash(&self, relative: &str, verify: bool) -> Option<String> {
        if !hashable(relative) {
            return None;
        }
        let recorded = self
            .entries
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(relative)
            .cloned();
        match recorded {
            Some(entry) if !verify => Some(entry.hash),
            Some(entry) => match regular_file(&self.directory, relative) {
                Some(metadata) if entry.describes(&metadata) => Some(entry.hash),
                _ => self.rehash(relative),
            },
            None if self.follow_edits => self.rehash(relative),
            None => None,
        }
    }

    /// Hashes `relative` again and records the result, or forgets the file
    /// when it is gone or no longer qualifies.
    fn rehash(&self, relative: &str) -> Option<String> {
        let fresh = regular_file(&self.directory, relative)
            .and_then(|_| hash_file(&self.directory.join(relative), self.limits.file_bytes));
        let mut entries = self.entries.write().unwrap_or_else(PoisonError::into_inner);
        match fresh {
            Some(entry) if entries.contains_key(relative) || entries.len() < self.limits.files => {
                let hash = entry.hash.clone();
                entries.insert(relative.to_owned(), entry);
                Some(hash)
            }
            _ => {
                entries.remove(relative);
                None
            }
        }
    }
}

/// The metadata of `directory/relative` when it is a regular file and no
/// component below `directory` is a symbolic link.
fn regular_file(directory: &Path, relative: &str) -> Option<Metadata> {
    let mut path = directory.to_path_buf();
    let mut metadata = None;
    for segment in relative.split('/') {
        path.push(segment);
        let current = fs::symlink_metadata(&path).ok()?;
        if current.file_type().is_symlink() {
            return None;
        }
        metadata = Some(current);
    }
    metadata.filter(Metadata::is_file)
}

/// The entry of the regular file at `path`, or `None` when it cannot be read
/// or is larger than `max_bytes`.
fn hash_file(path: &Path, max_bytes: u64) -> Option<Entry> {
    let mut file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return None;
    }
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = [0_u8; 65_536];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        context.update(&buffer[..read]);
    }
    let digest = context.finish();
    let hash = digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Some(Entry {
        hash: hash[..HASH_LEN].to_owned(),
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

/// Records the files below `directory` (`prefix` is its path below
/// `static/`), in name order so the file limit keeps the same files.
fn collect(
    directory: &Path,
    prefix: &str,
    depth: usize,
    limits: Limits,
    entries: &mut HashMap<String, Entry>,
) {
    let Ok(read) = fs::read_dir(directory) else {
        return;
    };
    let mut children: Vec<_> = read.filter_map(Result::ok).collect();
    children.sort_by_key(fs::DirEntry::file_name);
    for child in children {
        if entries.len() >= limits.files {
            return;
        }
        let Ok(name) = child.file_name().into_string() else {
            continue;
        };
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        // `DirEntry::file_type` does not follow symbolic links.
        let Ok(kind) = child.file_type() else {
            continue;
        };
        if kind.is_dir() && depth < limits.depth && plain_segment(&name) {
            collect(&child.path(), &relative, depth + 1, limits, entries);
        } else if kind.is_file()
            && hashable(&relative)
            && let Some(entry) = hash_file(&child.path(), limits.file_bytes)
        {
            entries.insert(relative, entry);
        }
    }
}
