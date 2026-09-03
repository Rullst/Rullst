//! Bounded metadata-only cache inspection contracts.

/// Maximum entries returned by one cache inspection.
pub const MAX_CACHE_INSPECTION_ENTRIES: usize = 200;

/// Metadata for one cache entry. Values are deliberately never included.
#[derive(Clone, PartialEq, Eq)]
pub struct CacheEntryMetadata {
    pub(crate) logical_key: String,
    pub(crate) value_bytes: usize,
    pub(crate) remaining_ttl_ms: Option<u64>,
}

impl std::fmt::Debug for CacheEntryMetadata {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CacheEntryMetadata")
            .field("logical_key", &"[REDACTED]")
            .field("value_bytes", &self.value_bytes)
            .field("remaining_ttl_ms", &self.remaining_ttl_ms)
            .finish()
    }
}

impl CacheEntryMetadata {
    pub(crate) fn new(
        logical_key: String,
        value_bytes: usize,
        remaining_ttl_ms: Option<u64>,
    ) -> Self {
        Self {
            logical_key,
            value_bytes,
            remaining_ttl_ms,
        }
    }

    /// Returns the exact logical key.
    ///
    /// Keys can contain application identifiers. Keep this output inside an
    /// explicitly authorized local diagnostic boundary.
    pub fn logical_key(&self) -> &str {
        &self.logical_key
    }

    /// Exact UTF-8 byte length of the cached value without returning it.
    pub const fn value_bytes(&self) -> usize {
        self.value_bytes
    }

    /// Approximate remaining TTL in milliseconds, or `None` for no expiry.
    pub const fn remaining_ttl_ms(&self) -> Option<u64> {
        self.remaining_ttl_ms
    }
}

/// One bounded metadata-only cache snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheInspection {
    pub(crate) entries: Vec<CacheEntryMetadata>,
    pub(crate) truncated: bool,
}

impl CacheInspection {
    pub(crate) fn new(entries: Vec<CacheEntryMetadata>, truncated: bool) -> Self {
        Self { entries, truncated }
    }

    /// Entries in deterministic logical-key order.
    pub fn entries(&self) -> &[CacheEntryMetadata] {
        &self.entries
    }

    /// Whether additional entries existed beyond the requested bound.
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

pub(crate) fn validate_inspection_limit(limit: usize) -> Result<(), super::CacheError> {
    if limit == 0 || limit > MAX_CACHE_INSPECTION_ENTRIES {
        return Err(super::CacheError::InvalidInspectionLimit);
    }
    Ok(())
}
