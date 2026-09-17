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
}

pub(super) fn load() -> Result<CachedCatalog, CacheError> {
    platform::load(now()?)
}

pub(super) fn store(body: &[u8]) -> Result<(), CacheError> {
    platform::store(body, now()?)
}
