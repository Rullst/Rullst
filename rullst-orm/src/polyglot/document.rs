use async_trait::async_trait;

use super::PolyglotError;

const MAX_IDENTIFIER_BYTES: usize = 64;
const MAX_DOCUMENT_ID_BYTES: usize = 128;
const MAX_PAGE_SIZE: u32 = 500;

/// A validated portable collection or table name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CollectionName(String);

impl CollectionName {
    /// Validates an ASCII identifier before it can reach a driver.
    pub fn new(value: impl Into<String>) -> Result<Self, PolyglotError> {
        let value = value.into();
        let mut bytes = value.bytes();
        let starts_safely = bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
        let rest_is_safe = bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
        if value.len() > MAX_IDENTIFIER_BYTES || !starts_safely || !rest_is_safe {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "collection name",
                reason: "use 1-64 ASCII letters, digits, or underscores and start with a letter or underscore",
            });
        }
        Ok(Self(value))
    }

    /// Returns the validated identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated portable document identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentId(String);

impl DocumentId {
    /// Validates a bounded identifier accepted by every document adapter.
    pub fn new(value: impl Into<String>) -> Result<Self, PolyglotError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_DOCUMENT_ID_BYTES
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
        if !valid {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "document id",
                reason: "use 1-128 ASCII letters, digits, underscores, or hyphens",
            });
        }
        Ok(Self(value))
    }

    /// Returns the validated identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A bounded, deterministic document-list page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentPage {
    offset: u64,
    limit: u32,
}

impl DocumentPage {
    /// Creates a page with a maximum size of 500 documents.
    pub fn new(offset: u64, limit: u32) -> Result<Self, PolyglotError> {
        if limit == 0
            || limit > MAX_PAGE_SIZE
            || offset > i64::MAX as u64
            || usize::try_from(offset).is_err()
        {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "document page",
                reason: "offset must fit the target index and a signed 64-bit integer; limit must be between 1 and 500",
            });
        }
        Ok(Self { offset, limit })
    }

    /// Returns the zero-based number of documents to skip.
    pub const fn offset(self) -> u64 {
        self.offset
    }

    /// Returns the bounded number of documents to read.
    pub const fn limit(self) -> u32 {
        self.limit
    }
}

/// Portable document CRUD without pretending to expose relational semantics.
#[async_trait]
pub trait DocumentRepository<T>: Send + Sync {
    /// Creates a document and fails on an existing identifier.
    async fn create(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        entity: &T,
    ) -> Result<(), PolyglotError>;

    /// Finds one document by its portable identifier.
    async fn find(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
    ) -> Result<Option<T>, PolyglotError>;

    /// Replaces an existing document and fails if it does not exist.
    async fn replace(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
        entity: &T,
    ) -> Result<(), PolyglotError>;

    /// Deletes a document, returning whether it existed.
    async fn delete(
        &self,
        collection: &CollectionName,
        id: &DocumentId,
    ) -> Result<bool, PolyglotError>;

    /// Lists a deterministic, bounded page ordered by document identifier.
    async fn list(
        &self,
        collection: &CollectionName,
        page: DocumentPage,
    ) -> Result<Vec<T>, PolyglotError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_portable_identifiers_and_pages() {
        assert_eq!(
            CollectionName::new("audit_events").unwrap().as_str(),
            "audit_events"
        );
        assert!(CollectionName::new("events-v2").is_err());
        assert_eq!(DocumentId::new("evt-42").unwrap().as_str(), "evt-42");
        assert!(DocumentId::new("../secret").is_err());
        assert_eq!(DocumentPage::new(10, 25).unwrap().limit(), 25);
        assert!(DocumentPage::new(0, 0).is_err());
        assert!(DocumentPage::new(0, 501).is_err());
        assert!(DocumentPage::new(u64::MAX, 1).is_err());
    }
}
