//! Advisory catalog cache. Never an artifact, signature or installation authority.
//! Fail closed on platforms without an implemented private-directory verifier.
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const MAX_AGE_SECONDS: u64 = 6 * 60 * 60;

#[cfg(any(unix, windows))]
#[path = "cache/codec.rs"]
mod codec;

#[derive(Debug, thiserror::Error)]
pub(super) enum CacheError {
    #[error("{0}")]
    Invalid(&'static str),
    #[cfg(any(unix, windows))]
    #[error("catalog cache I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub(super) struct CachedCatalog {
    pub body: Vec<u8>,
    pub age_seconds: u64,
}

#[cfg(any(unix, windows))]
struct UnlockOnDrop<'a>(&'a std::fs::File);

#[cfg(any(unix, windows))]
impl Drop for UnlockOnDrop<'_> {
    fn drop(&mut self) {
        // Close-on-exec does not prevent transient inheritance during a fork
        // in another thread. Unlock explicitly rather than waiting for every
        // duplicate of the open file description to close.
        let _ = self.0.unlock();
    }
}

fn now() -> Result<u64, CacheError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| CacheError::Invalid("system clock precedes the Unix epoch"))
}

#[cfg(unix)]
#[path = "cache/unix.rs"]
mod platform;

#[cfg(windows)]
#[path = "cache/windows.rs"]
mod platform;

#[cfg(not(any(unix, windows)))]
mod platform {
    use super::{CacheError, CachedCatalog};

    pub(super) fn load(_now: u64) -> Result<CachedCatalog, CacheError> {
        Err(CacheError::Invalid(
            "private catalog caching is not available on this platform yet; use online discovery",
        ))
    }

    pub(super) fn store(_body: &[u8], _now: u64) -> Result<(), CacheError> {
        Err(CacheError::Invalid(
            "private catalog caching is not available on this platform yet",
        ))
    }

    pub(super) fn verification_manifest(
        _body: &[u8],
    ) -> Result<tempfile::NamedTempFile, CacheError> {
        Err(CacheError::Invalid(
            "private artifact verification is unavailable on this platform",
        ))
    }

    pub(super) fn project_workspace() -> Result<std::path::PathBuf, CacheError> {
        Err(CacheError::Invalid(
            "private project preparation is unavailable on this platform",
        ))
    }

    pub(super) fn open_project(
        _path: &std::path::Path,
    ) -> Result<(std::path::PathBuf, std::fs::File), CacheError> {
        Err(CacheError::Invalid(
            "private project verification is unavailable on this platform",
        ))
    }

    pub(super) fn installation_root(
        _path: &std::path::Path,
    ) -> Result<std::path::PathBuf, CacheError> {
        Err(CacheError::Invalid(
            "private CLI installation is unavailable on this platform",
        ))
    }

    pub(super) fn installation_file(_path: &std::path::Path) -> Result<(), CacheError> {
        Err(CacheError::Invalid(
            "private CLI installation is unavailable on this platform",
        ))
    }

    pub(super) fn create_installation_directory(
        _path: &std::path::Path,
    ) -> Result<std::path::PathBuf, CacheError> {
        Err(CacheError::Invalid(
            "private installation is unavailable on this platform",
        ))
    }

    pub(super) fn new_installation_file(
        _path: &std::path::Path,
    ) -> Result<std::fs::File, CacheError> {
        Err(CacheError::Invalid(
            "private installation is unavailable on this platform",
        ))
    }

    pub(super) fn installation_lock(_path: &std::path::Path) -> Result<std::fs::File, CacheError> {
        Err(CacheError::Invalid(
            "private installation is unavailable on this platform",
        ))
    }

    pub(super) fn source_lock(_name: &str) -> Result<std::fs::File, CacheError> {
        Err(CacheError::Invalid(
            "private source locking is unavailable on this platform",
        ))
    }
}

pub(super) fn load() -> Result<CachedCatalog, CacheError> {
    platform::load(now()?)
}

pub(super) fn store(body: &[u8]) -> Result<(), CacheError> {
    platform::store(body, now()?)
}

// Shares only the private filesystem boundary, never cached catalog authority.
pub(super) fn verification_manifest(body: &[u8]) -> Result<tempfile::NamedTempFile, CacheError> {
    if body.is_empty() || body.len() > 16 * 1024 {
        return Err(CacheError::Invalid("invalid verification manifest size"));
    }
    platform::verification_manifest(body)
}

pub(super) fn installation_file(path: &std::path::Path) -> Result<(), CacheError> {
    platform::installation_file(path)
}

pub(super) fn create_installation_directory(
    path: &std::path::Path,
) -> Result<std::path::PathBuf, CacheError> {
    platform::create_installation_directory(path)
}

pub(super) fn new_installation_file(path: &std::path::Path) -> Result<std::fs::File, CacheError> {
    platform::new_installation_file(path)
}

pub(super) fn installation_lock(path: &std::path::Path) -> Result<FileLease, CacheError> {
    acquire(
        platform::installation_lock(path)?,
        "installation destination is busy",
    )
}

pub(super) struct PrivateWorkspace {
    path: std::path::PathBuf,
    retained: bool,
}

impl PrivateWorkspace {
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn retain(mut self) -> std::path::PathBuf {
        self.retained = true;
        self.path.clone()
    }
}

impl Drop for PrivateWorkspace {
    fn drop(&mut self) {
        if !self.retained {
            // Constructed only from a fresh private directory, never from a
            // serialized or caller-supplied path. Same-user attacks are outside
            // this boundary; removal does not follow contained symlinks.
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

pub(super) fn project_workspace() -> Result<PrivateWorkspace, CacheError> {
    Ok(PrivateWorkspace {
        path: platform::project_workspace()?,
        retained: false,
    })
}

pub(super) struct LockedProject {
    pub path: std::path::PathBuf,
    _lease: FileLease,
}

pub(super) struct FileLease(std::fs::File);

impl Drop for FileLease {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

pub(super) fn open_project(path: &std::path::Path) -> Result<LockedProject, CacheError> {
    let (path, lock) = platform::open_project(path)?;
    Ok(LockedProject {
        path,
        _lease: acquire(lock, "prepared project is busy")?,
    })
}

fn acquire(lock: std::fs::File, busy_message: &'static str) -> Result<FileLease, CacheError> {
    match lock.try_lock() {
        Ok(()) => Ok(FileLease(lock)),
        Err(std::fs::TryLockError::WouldBlock) => Err(CacheError::Invalid(busy_message)),
        Err(std::fs::TryLockError::Error(error)) => {
            #[cfg(any(unix, windows))]
            {
                Err(error.into())
            }
            #[cfg(not(any(unix, windows)))]
            {
                let _ = error;
                Err(CacheError::Invalid("project lock failed"))
            }
        }
    }
}

pub(super) fn source_lock(root: &std::path::Path) -> Result<FileLease, CacheError> {
    use sha2::{Digest, Sha256};
    #[cfg(any(unix, windows))]
    let canonical = root.canonicalize()?;
    #[cfg(not(any(unix, windows)))]
    let canonical = root.to_path_buf();
    let name = format!(
        "source-{}.lock",
        hex::encode(Sha256::digest(canonical.as_os_str().as_encoded_bytes()))
    );
    acquire(platform::source_lock(&name)?, "prepared project is busy")
}

#[cfg(any(unix, windows))]
fn project_path(
    requested: &std::path::Path,
    directory: &std::path::Path,
) -> Result<std::path::PathBuf, CacheError> {
    let name = requested
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(CacheError::Invalid(
            "select an existing private project preparation",
        ))?;
    let identifier = name
        .strip_prefix("project-")
        .ok_or(CacheError::Invalid("invalid preparation name"))?;
    if uuid::Uuid::parse_str(identifier)
        .map(|id| id.to_string() != identifier)
        .unwrap_or(true)
        || requested
            .parent()
            .ok_or(CacheError::Invalid("missing preparation parent"))?
            .canonicalize()?
            != directory.canonicalize()?
    {
        return Err(CacheError::Invalid(
            "preparation must belong to the configured private update cache",
        ));
    }
    Ok(directory.join(name))
}

#[cfg(windows)]
pub(super) fn private_file_descriptor() -> Result<String, CacheError> {
    platform::private_file_descriptor()
}

pub(super) fn installation_root(path: &std::path::Path) -> Result<std::path::PathBuf, CacheError> {
    platform::installation_root(path)
}
