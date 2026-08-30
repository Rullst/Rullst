//! Bounded process-local cosine retriever for offline development and tests.

use super::{RagDocument, RagRetrievalError, RagRetriever};
use crate::ai::cosine_similarity;
use async_trait::async_trait;
use rullst_core::security::TenantContext;
use std::collections::HashMap;
use std::sync::Mutex;

const MAX_INDEXED_DOCUMENTS: usize = 100_000;
const MAX_VECTOR_DIMENSIONS: usize = 65_536;
const MAX_VECTOR_COMPONENT: f32 = 1_000_000.0;

#[derive(Debug, Clone)]
struct IndexedDocument {
    document: RagDocument,
    vector: Vec<f32>,
}

/// Bounded tenant-partitioned in-memory cosine retriever.
///
/// This is deterministic process-local infrastructure for development, tests, and small ephemeral
/// workloads. It is not durable or distributed. Production applications can implement
/// [`RagRetriever`] over Rullst ORM pgvector/Qdrant while retaining the same pipeline contract.
#[derive(Debug)]
pub struct InMemoryRagRetriever {
    capacity: usize,
    dimensions: usize,
    documents: Mutex<HashMap<(String, String), IndexedDocument>>,
}

impl InMemoryRagRetriever {
    /// Creates an empty retriever with exact vector dimensions and a hard document capacity.
    pub fn try_new(capacity: usize, dimensions: usize) -> Result<Self, RagRetrievalError> {
        if !(1..=MAX_INDEXED_DOCUMENTS).contains(&capacity) {
            return Err(RagRetrievalError(format!(
                "in-memory RAG capacity must be between 1 and {MAX_INDEXED_DOCUMENTS}"
            )));
        }
        if !(1..=MAX_VECTOR_DIMENSIONS).contains(&dimensions) {
            return Err(RagRetrievalError(format!(
                "RAG vector dimensions must be between 1 and {MAX_VECTOR_DIMENSIONS}"
            )));
        }
        Ok(Self {
            capacity,
            dimensions,
            documents: Mutex::new(HashMap::new()),
        })
    }

    /// Inserts or replaces a document within the selected tenant partition.
    pub fn upsert(
        &self,
        tenant: &TenantContext,
        document: RagDocument,
        vector: Vec<f32>,
    ) -> Result<(), RagRetrievalError> {
        if document.tenant_id() != tenant.tenant_id {
            return Err(RagRetrievalError(
                "cannot index a document under a different tenant".to_string(),
            ));
        }
        validate_vector(&vector, self.dimensions)?;
        let key = (tenant.tenant_id.clone(), document.id().to_string());
        let mut documents = self
            .documents
            .lock()
            .map_err(|_| RagRetrievalError("in-memory RAG index lock was poisoned".to_string()))?;
        if !documents.contains_key(&key) && documents.len() == self.capacity {
            return Err(RagRetrievalError(
                "in-memory RAG document capacity reached".to_string(),
            ));
        }
        documents.insert(key, IndexedDocument { document, vector });
        Ok(())
    }

    /// Removes one document from the selected tenant partition.
    pub fn remove(
        &self,
        tenant: &TenantContext,
        document_id: &str,
    ) -> Result<bool, RagRetrievalError> {
        self.documents
            .lock()
            .map(|mut documents| {
                documents
                    .remove(&(tenant.tenant_id.clone(), document_id.to_string()))
                    .is_some()
            })
            .map_err(|_| RagRetrievalError("in-memory RAG index lock was poisoned".to_string()))
    }

    /// Counts documents visible in exactly one tenant partition.
    pub fn len_for_tenant(&self, tenant: &TenantContext) -> Result<usize, RagRetrievalError> {
        self.documents
            .lock()
            .map(|documents| {
                documents
                    .keys()
                    .filter(|(tenant_id, _)| tenant_id == &tenant.tenant_id)
                    .count()
            })
            .map_err(|_| RagRetrievalError("in-memory RAG index lock was poisoned".to_string()))
    }
}

#[async_trait]
impl RagRetriever for InMemoryRagRetriever {
    async fn retrieve(
        &self,
        tenant: &TenantContext,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<RagDocument>, RagRetrievalError> {
        validate_vector(query_embedding, self.dimensions)?;
        if limit == 0 || limit > 32 {
            return Err(RagRetrievalError(
                "RAG retrieval limit must be between 1 and 32".to_string(),
            ));
        }
        let documents = self
            .documents
            .lock()
            .map_err(|_| RagRetrievalError("in-memory RAG index lock was poisoned".to_string()))?;
        let mut matches = documents
            .iter()
            .filter(|((tenant_id, _), _)| tenant_id == &tenant.tenant_id)
            .map(|(_, indexed)| {
                let score = cosine_similarity(query_embedding, &indexed.vector);
                indexed.document.clone().with_score(score)
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| right.score().total_cmp(&left.score()));
        matches.truncate(limit);
        Ok(matches)
    }
}

fn validate_vector(vector: &[f32], dimensions: usize) -> Result<(), RagRetrievalError> {
    if vector.len() != dimensions {
        return Err(RagRetrievalError(format!(
            "expected {dimensions} vector dimensions, received {}",
            vector.len()
        )));
    }
    if !vector
        .iter()
        .all(|value| value.is_finite() && value.abs() <= MAX_VECTOR_COMPONENT)
    {
        return Err(RagRetrievalError(format!(
            "vector components must be finite and within ±{MAX_VECTOR_COMPONENT}"
        )));
    }
    Ok(())
}
