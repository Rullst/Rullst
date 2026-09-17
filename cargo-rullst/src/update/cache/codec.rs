use super::{CacheError, CachedCatalog, MAX_AGE_SECONDS};
use crate::ui::update_check::CATALOG_LIMIT;

pub(super) const HEADER: &[u8] = b"rullst-update-cache-v1\n";
pub(super) const FILE_LIMIT: u64 = CATALOG_LIMIT + 64;

pub(super) fn decode(bytes: &[u8], now: u64) -> Result<CachedCatalog, CacheError> {
    let data = bytes
        .strip_prefix(HEADER)
        .ok_or(CacheError::Invalid("unsupported catalog cache format"))?;
    let delimiter = data
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or(CacheError::Invalid("missing catalog cache timestamp"))?;
    let timestamp = &data[..delimiter];
    if timestamp.is_empty() || timestamp.len() > 20 || !timestamp.iter().all(u8::is_ascii_digit) {
        return Err(CacheError::Invalid("invalid catalog cache timestamp"));
    }
    let timestamp: u64 = std::str::from_utf8(timestamp)
        .ok()
        .and_then(|v| v.parse().ok())
        .ok_or(CacheError::Invalid("invalid catalog cache timestamp"))?;
    let age_seconds = now
        .checked_sub(timestamp)
        .filter(|age| *age < MAX_AGE_SECONDS)
        .ok_or(CacheError::Invalid(
            "catalog cache is expired or dated in the future; refresh online",
        ))?;
    let body = &data[delimiter + 1..];
    if body.is_empty() || body.len() as u64 > CATALOG_LIMIT {
        return Err(CacheError::Invalid("invalid cached catalog size"));
    }
    Ok(CachedCatalog {
        body: body.to_vec(),
        age_seconds,
    })
}
