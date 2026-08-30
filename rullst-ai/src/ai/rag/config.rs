//! Validated context budgets for the RAG pipeline.

use super::pipeline::RagError;

const MAX_DOCUMENTS: usize = 32;
const MAX_DOCUMENT_CHARS: usize = 32 * 1024;
const MAX_CONTEXT_CHARS: usize = 128 * 1024;

/// Bounded knobs for a [`super::RagPipeline`] operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RagConfig {
    pub(super) max_documents: usize,
    pub(super) max_document_chars: usize,
    pub(super) max_context_chars: usize,
}

impl RagConfig {
    /// Creates a validated context budget.
    pub fn try_new(
        max_documents: usize,
        max_document_chars: usize,
        max_context_chars: usize,
    ) -> Result<Self, RagError> {
        if !(1..=MAX_DOCUMENTS).contains(&max_documents) {
            return Err(RagError::InvalidConfiguration(format!(
                "max_documents must be between 1 and {MAX_DOCUMENTS}"
            )));
        }
        if !(1..=MAX_DOCUMENT_CHARS).contains(&max_document_chars) {
            return Err(RagError::InvalidConfiguration(format!(
                "max_document_chars must be between 1 and {MAX_DOCUMENT_CHARS}"
            )));
        }
        if !(1..=MAX_CONTEXT_CHARS).contains(&max_context_chars) {
            return Err(RagError::InvalidConfiguration(format!(
                "max_context_chars must be between 1 and {MAX_CONTEXT_CHARS}"
            )));
        }
        Ok(Self {
            max_documents,
            max_document_chars,
            max_context_chars,
        })
    }

    /// Maximum documents requested from and accepted from the retriever.
    pub fn max_documents(self) -> usize {
        self.max_documents
    }

    /// Maximum Unicode scalar count included from one document.
    pub fn max_document_chars(self) -> usize {
        self.max_document_chars
    }

    /// Maximum Unicode scalar count included across all documents.
    pub fn max_context_chars(self) -> usize {
        self.max_context_chars
    }
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            max_documents: 8,
            max_document_chars: 8 * 1024,
            max_context_chars: 32 * 1024,
        }
    }
}
