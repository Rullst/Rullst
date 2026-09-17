#[cfg(test)]
use super::MAX_AGE_SECONDS;
use super::codec::{FILE_LIMIT, HEADER, decode};
use super::{CacheError, CachedCatalog};
use crate::ui::update_check::CATALOG_LIMIT;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Component, Path, PathBuf},
};

const CATALOG: &str = "catalog-v1";

// These operations assume no hostile same-user/root process. The parent must
// be caller-owned and not group/world-writable; the final directory is private.
// No path supplied by catalog contents participates in filesystem operations.
fn base_directory(create: bool) -> Result<PathBuf, CacheError> {
    let base = if let Some(path) = std::env::var_os("XDG_CACHE_HOME").filter(|v| !v.is_empty()) {
        PathBuf::from(path)
    } else {
        let home = std::env::var_os("HOME")
            .ok_or(CacheError::Invalid("no user cache directory is configured"))?;
        let home = PathBuf::from(home);
        validate_absolute(&home)?;
        // macOS may have OS-managed aliases above the user's actual home.
        let home = home.canonicalize()?;
        validate_directory(&home, false)?;
        home.join(".cache")
    };
    validate_absolute(&base)?;
    if create && !base.try_exists()? {
        let parent = base
            .parent()
            .ok_or(CacheError::Invalid("invalid cache parent"))?;
        validate_directory(parent, false)?;
        create_private_directory(&base)?;
    }
    // Resolve only the caller-selected base, not the private cache or its files.
    // An untrusted writable/foreign-owned base still fails validation.
    let base = base.canonicalize()?;
    validate_directory(&base, false)?;
    Ok(base)
}

fn validate_absolute(path: &Path) -> Result<(), CacheError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(CacheError::Invalid(
            "cache directory must be an absolute path without traversal",
        ));
    }
    Ok(())
}

fn validate_directory(path: &Path, private: bool) -> Result<(), CacheError> {
    let metadata = fs::symlink_metadata(path)?;
    let prohibited = if private { 0o077 } else { 0o022 };
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & prohibited != 0
    {
        return Err(CacheError::Invalid(
            "cache directory has an unsafe owner, type or permissions",
        ));
    }
    Ok(())
}

fn create_private_directory(path: &Path) -> Result<(), CacheError> {
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn cache_directory(base: &Path, create: bool) -> Result<PathBuf, CacheError> {
    // An otherwise private leaf is not trustworthy inside another user's
    // replaceable parent. Permit root's sticky /tmp, but not arbitrary writers.
    for ancestor in base.ancestors() {
        let metadata = fs::symlink_metadata(ancestor)?;
        let owner = metadata.uid();
        let root_sticky = owner == 0 && metadata.mode() & 0o1000 != 0;
        if !metadata.is_dir()
            || (owner != 0 && owner != rustix::process::geteuid().as_raw())
            || (metadata.mode() & 0o022 != 0 && !root_sticky)
        {
            return Err(CacheError::Invalid("unsafe catalog cache ancestor"));
        }
    }
    validate_directory(base, false)?;
    let path = base.join("rullst-update-v1");
    if create {
        create_private_directory(&path)?;
    }
    validate_directory(&path, true)?;
    Ok(path)
}

fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    // Reject symlinks at open, and avoid blocking on an attacker-supplied FIFO.
    options
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32);
    options.mode(0o600);
    options
}

fn validate_file(file: &File) -> Result<(), CacheError> {
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
        || metadata.nlink() != 1
        || metadata.len() > FILE_LIMIT
    {
        return Err(CacheError::Invalid(
            "catalog cache has an unsafe owner, type, links, size or permissions",
        ));
    }
    Ok(())
}

fn load_at(base: &Path, now: u64) -> Result<CachedCatalog, CacheError> {
    let directory = cache_directory(base, false)?;
    let file = options().read(true).open(directory.join(CATALOG))?;
    validate_file(&file)?;
    let mut bytes = Vec::new();
    file.take(FILE_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > FILE_LIMIT {
        return Err(CacheError::Invalid("catalog cache exceeds its size limit"));
    }
    decode(&bytes, now)
}

fn store_at(base: &Path, body: &[u8], now: u64) -> Result<(), CacheError> {
    if body.is_empty() || body.len() as u64 > CATALOG_LIMIT {
        return Err(CacheError::Invalid("invalid catalog size for cache"));
    }
    let directory = cache_directory(base, true)?;
    let lock = options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("catalog.lock"))?;
    validate_file(&lock)?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => {
            return Err(CacheError::Invalid(
                "catalog cache is busy; no wait or overwrite attempted",
            ));
        }
        Err(fs::TryLockError::Error(error)) => return Err(error.into()),
    }
    let _unlock = super::UnlockOnDrop(&lock);
    let mut staged = tempfile::Builder::new()
        .prefix("catalog-stage-")
        .tempfile_in(&directory)?;
    staged.write_all(HEADER)?;
    writeln!(staged, "{now}")?;
    staged.write_all(body)?;
    staged.as_file().sync_all()?;
    // Atomic directory-entry replacement, never truncate/follow a prior target.
    staged
        .persist(directory.join(CATALOG))
        .map_err(|error| error.error)?;
    File::open(&directory)?.sync_all()?;
    Ok(())
}

pub(super) fn load(now: u64) -> Result<CachedCatalog, CacheError> {
    load_at(&base_directory(false)?, now)
}

pub(super) fn store(body: &[u8], now: u64) -> Result<(), CacheError> {
    store_at(&base_directory(true)?, body, now)
}

pub(super) fn verification_manifest(body: &[u8]) -> Result<tempfile::NamedTempFile, CacheError> {
    let directory = cache_directory(&base_directory(true)?, true)?;
    let mut file = tempfile::Builder::new()
        .prefix("verify-manifest-")
        .tempfile_in(directory)?;
    validate_file(file.as_file())?;
    file.write_all(body)?;
    file.as_file().sync_all()?;
    Ok(file)
}

pub(super) fn project_workspace() -> Result<PathBuf, CacheError> {
    let directory = cache_directory(&base_directory(true)?, true)?;
    let path = directory.join(format!("project-{}", uuid::Uuid::new_v4()));
    fs::DirBuilder::new().mode(0o700).create(&path)?;
    validate_directory(&path, true)?;
    Ok(path)
}

pub(super) fn open_project(requested: &Path) -> Result<(PathBuf, File), CacheError> {
    let directory = cache_directory(&base_directory(false)?, false)?;
    let path = super::project_path(requested, &directory)?;
    validate_directory(&path, true)?;
    let lock = options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.join("operation.lock"))?;
    validate_file(&lock)?;
    Ok((path, lock))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
