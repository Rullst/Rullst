use super::{
    CacheError, CachedCatalog,
    codec::{FILE_LIMIT, HEADER, decode},
};
use crate::ui::update_check::CATALOG_LIMIT;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::windows::fs::OpenOptionsExt,
    path::{Component, Path, PathBuf, Prefix},
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, READ_CONTROL,
};

#[path = "windows_acl.rs"]
mod acl;
use acl::Identity;

const CATALOG: &str = "catalog-v1";

fn validate_path(path: &Path) -> Result<(), CacheError> {
    if !path.is_absolute()
        || !matches!(path.components().next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        || path.components().any(|part| match part {
            Component::ParentDir | Component::CurDir => true,
            Component::Normal(value) => value.to_string_lossy().contains([':', '\0']),
            _ => false,
        })
    {
        return Err(CacheError::Invalid(
            "cache path must be a local absolute drive path without traversal or streams",
        ));
    }
    Ok(())
}

fn directory_handle(path: &Path) -> Result<File, CacheError> {
    Ok(OpenOptions::new()
        .access_mode(READ_CONTROL | FILE_READ_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?)
}

fn cache_directory(base: &Path, create: bool, identity: &Identity) -> Result<PathBuf, CacheError> {
    validate_path(base)?;
    for ancestor in base.ancestors() {
        identity.validate(&directory_handle(ancestor)?, ancestor != base, false, true)?;
    }
    let path = base.join("rullst-update-v1");
    if create {
        identity.create_directory(&path)?;
    }
    identity.validate(&directory_handle(&path)?, false, true, true)?;
    Ok(path)
}

fn options(write: bool) -> OpenOptions {
    let mut options = OpenOptions::new();
    // The standard read/write access includes READ_CONTROL. OPEN_REPARSE_POINT
    // exposes a link itself so validation rejects it before any content I/O.
    options
        .read(true)
        .write(write)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    options
}

fn validate_file(file: &File, identity: &Identity) -> Result<(), CacheError> {
    identity.validate(file, false, true, false)?;
    if file.metadata()?.len() > FILE_LIMIT {
        return Err(CacheError::Invalid("catalog cache exceeds its size limit"));
    }
    Ok(())
}

fn load_at(base: &Path, now: u64) -> Result<CachedCatalog, CacheError> {
    let identity = Identity::current()?;
    let directory = cache_directory(base, false, &identity)?;
    let file = options(false).open(directory.join(CATALOG))?;
    validate_file(&file, &identity)?;
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
    let identity = Identity::current()?;
    let directory = cache_directory(base, true, &identity)?;
    let lock = options(true)
        .create(true)
        .truncate(false)
        .open(directory.join("catalog.lock"))?;
    validate_file(&lock, &identity)?;
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
    validate_file(staged.as_file(), &identity)?;
    staged.write_all(HEADER)?;
    writeln!(staged, "{now}")?;
    staged.write_all(body)?;
    staged.as_file().sync_all()?;
    staged
        .persist(directory.join(CATALOG))
        .map_err(|error| error.error)?;
    Ok(())
}

fn base_directory() -> Result<PathBuf, CacheError> {
    let path = std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .ok_or(CacheError::Invalid("LOCALAPPDATA is not configured"))?;
    Ok(PathBuf::from(path))
}

pub(super) fn load(now: u64) -> Result<CachedCatalog, CacheError> {
    load_at(&base_directory()?, now)
}
pub(super) fn store(body: &[u8], now: u64) -> Result<(), CacheError> {
    store_at(&base_directory()?, body, now)
}

pub(super) fn verification_manifest(body: &[u8]) -> Result<tempfile::NamedTempFile, CacheError> {
    let identity = Identity::current()?;
    let directory = cache_directory(&base_directory()?, true, &identity)?;
    let mut file = tempfile::Builder::new()
        .prefix("verify-manifest-")
        .tempfile_in(directory)?;
    validate_file(file.as_file(), &identity)?;
    file.write_all(body)?;
    file.as_file().sync_all()?;
    Ok(file)
}

pub(super) fn project_workspace() -> Result<PathBuf, CacheError> {
    let identity = Identity::current()?;
    let directory = cache_directory(&base_directory()?, true, &identity)?;
    let path = directory.join(format!("project-{}", uuid::Uuid::new_v4()));
    if path.try_exists()? {
        return Err(CacheError::Invalid(
            "private project directory already exists",
        ));
    }
    identity.create_directory(&path)?;
    identity.validate(&directory_handle(&path)?, false, true, true)?;
    Ok(path)
}

pub(super) fn open_project(requested: &Path) -> Result<(PathBuf, File), CacheError> {
    let identity = Identity::current()?;
    let directory = cache_directory(&base_directory()?, false, &identity)?;
    let path = super::project_path(requested, &directory)?;
    identity.validate(&directory_handle(&path)?, false, true, true)?;
    let lock = options(true)
        .create(true)
        .truncate(false)
        .open(path.join("operation.lock"))?;
    validate_file(&lock, &identity)?;
    Ok((path, lock))
}

pub(super) fn source_lock(name: &str) -> Result<File, CacheError> {
    let identity = Identity::current()?;
    let directory = cache_directory(&base_directory()?, false, &identity)?;
    let file = options(true)
        .create(true)
        .truncate(false)
        .open(directory.join(name))?;
    validate_file(&file, &identity)?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    const BODY: &[u8] = b"{\"versions\":[]}";

    fn fixture() -> tempfile::TempDir {
        // Keep fixtures under the actual user profile, not a shared runner drive.
        tempfile::tempdir_in(base_directory().unwrap()).unwrap()
    }

    #[test]
    fn private_roundtrip_expiry_and_replacement() {
        let base = fixture();
        assert!(load_at(base.path(), 100).is_err());
        assert!(!base.path().join("rullst-update-v1").exists());
        store_at(base.path(), BODY, 100).unwrap();
        let cached = load_at(base.path(), 101).unwrap();
        assert_eq!(cached.body, BODY);
        assert_eq!(cached.age_seconds, 1);
        assert!(load_at(base.path(), 99).is_err());
        assert!(load_at(base.path(), 100 + super::super::MAX_AGE_SECONDS).is_err());
        store_at(base.path(), b"replacement", 102).unwrap();
        assert_eq!(load_at(base.path(), 102).unwrap().body, b"replacement");
    }

    #[test]
    fn busy_lock_hardlinks_and_broad_acl_fail_closed() {
        let base = fixture();
        store_at(base.path(), BODY, 100).unwrap();
        let directory = base.path().join("rullst-update-v1");
        let lock = options(true).open(directory.join("catalog.lock")).unwrap();
        lock.lock().unwrap();
        assert!(store_at(base.path(), b"new", 101).is_err());
        assert_eq!(load_at(base.path(), 101).unwrap().body, BODY);
        lock.unlock().unwrap();
        drop(lock);
        let link = directory.join("hardlink");
        fs::hard_link(directory.join(CATALOG), &link).unwrap();
        assert!(load_at(base.path(), 101).is_err());
        fs::remove_file(link).unwrap();
        assert!(
            std::process::Command::new("icacls")
                .arg(&directory)
                .args(["/grant", "*S-1-1-0:(OI)(CI)F"])
                .output()
                .unwrap()
                .status
                .success()
        );
        assert!(load_at(base.path(), 101).is_err());
        assert!(store_at(base.path(), b"new", 101).is_err());
    }

    #[test]
    fn existing_unprotected_directory_and_nonlocal_paths_are_rejected() {
        let base = fixture();
        fs::create_dir(base.path().join("rullst-update-v1")).unwrap();
        assert!(store_at(base.path(), BODY, 100).is_err());
        for path in [
            r"relative",
            r"\\server\share\cache",
            r"C:\cache\..\escape",
            r"C:\cache:stream",
        ] {
            assert!(validate_path(Path::new(path)).is_err());
        }
        assert!(store_at(base.path(), &[], 100).is_err());
        assert!(store_at(base.path(), &vec![0; CATALOG_LIMIT as usize + 1], 100).is_err());
    }
}

// Used only for a new lockfile that has no original access policy to preserve.
pub(super) fn private_file_descriptor() -> Result<String, CacheError> {
    Ok(Identity::current()?.private_file_descriptor())
}

pub(super) fn installation_root(requested: &Path) -> Result<PathBuf, CacheError> {
    validate_path(requested)?;
    let name = requested.file_name().ok_or(CacheError::Invalid(
        "installation needs a named destination directory",
    ))?;
    let parent = requested
        .parent()
        .ok_or(CacheError::Invalid("installation needs an existing parent"))?
        .canonicalize()?;
    let identity = Identity::current()?;
    for ancestor in parent.ancestors() {
        identity.validate(
            &directory_handle(ancestor)?,
            ancestor != parent,
            false,
            true,
        )?;
    }
    let root = parent.join(name);
    match fs::symlink_metadata(&root) {
        Ok(_) => identity.validate(&directory_handle(&root)?, false, true, true)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    Ok(root)
}

pub(super) fn installation_file(path: &Path) -> Result<(), CacheError> {
    Identity::current()?.validate(&options(false).open(path)?, false, true, false)
}
