//! Tenant-bound retrieval, context budgeting, guarded generation, and audit orchestration.

use super::{RagAuditEvent, RagAuditOutcome, RagAuditSink, RagConfig, build_rag_prompt};
use crate::ai::{AiClient, AiError, AiGuardrails};
use async_trait::async_trait;
use rullst_core::security::TenantContext;
use sha2::{Digest, Sha256};

const MAX_QUESTION_BYTES: usize = 8 * 1024;
const MAX_RETRIEVED_DOCUMENT_BYTES: usize = 64 * 1024;
const MAX_ANSWER_BYTES: usize = 64 * 1024;
const MAX_EMBEDDING_DIMENSIONS: usize = 65_536;
/// One tenant-tagged document returned by an application retriever.
#[derive(Debug, Clone, PartialEq)]
pub struct RagDocument {
    tenant_id: String,
    id: String,
    content: String,
    score: f32,
}

impl RagDocument {
    /// Creates a bounded document tagged with the trusted tenant context used for retrieval.
    pub fn try_new(
        tenant: &TenantContext,
        id: impl Into<String>,
        content: impl Into<String>,
        score: f32,
    ) -> Result<Self, RagError> {
        let id = id.into();
        if id.is_empty() || id.len() > 256 || id.chars().any(|character| character.is_control()) {
            return Err(RagError::InvalidDocument(
                "document ID must contain 1-256 bytes without control characters".to_string(),
            ));
        }
        let content = content.into();
        if content.trim().is_empty() || content.len() > MAX_RETRIEVED_DOCUMENT_BYTES {
            return Err(RagError::InvalidDocument(format!(
                "document content must contain 1-{MAX_RETRIEVED_DOCUMENT_BYTES} bytes"
            )));
        }
        if !score.is_finite() {
            return Err(RagError::InvalidDocument(
                "document score must be finite".to_string(),
            ));
        }
        Ok(Self {
            tenant_id: tenant.tenant_id.clone(),
            id,
            content,
            score,
        })
    }

    /// Returns the application document identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the retriever-defined finite relevance score.
    pub fn score(&self) -> f32 {
        self.score
    }

    /// Returns the trusted tenant tag attached during retrieval.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    pub(super) fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }
}

/// Citation metadata for a context passage included in the model request.
#[derive(Debug, Clone, PartialEq)]
pub struct RagSource {
    document_id: String,
    score: f32,
    included_chars: usize,
    truncated: bool,
}

impl RagSource {
    /// Returns the application document identifier.
    pub fn document_id(&self) -> &str {
        &self.document_id
    }

    /// Returns the retriever-defined score without assigning universal semantics to it.
    pub fn score(&self) -> f32 {
        self.score
    }

    /// Returns how many Unicode scalar values entered the context prompt.
    pub fn included_chars(&self) -> usize {
        self.included_chars
    }

    /// Reports whether the document was cut by a per-document or total context budget.
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}

/// Grounded answer plus the exact source identifiers selected for its prompt.
#[derive(Debug, Clone, PartialEq)]
pub struct RagAnswer {
    answer: String,
    sources: Vec<RagSource>,
}

impl RagAnswer {
    /// Returns the model answer.
    pub fn answer(&self) -> &str {
        &self.answer
    }

    /// Returns context source metadata in retriever order.
    pub fn sources(&self) -> &[RagSource] {
        &self.sources
    }
}

/// Typed failure returned by an application retriever.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("RAG retrieval failed: {0}")]
pub struct RagRetrievalError(pub String);

/// Tenant-aware vector/document retrieval boundary.
///
/// Implementations must apply authorization and tenant filtering in the authoritative datastore.
/// The pipeline additionally rejects every returned document whose tenant tag differs.
#[async_trait]
pub trait RagRetriever: Send + Sync {
    /// Returns at most `limit` documents in the backend's intended relevance order.
    async fn retrieve(
        &self,
        tenant: &TenantContext,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<RagDocument>, RagRetrievalError>;
}

/// Errors from validation, embedding, retrieval, generation, or required audit recording.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum RagError {
    /// A pipeline budget is invalid.
    #[error("invalid RAG configuration: {0}")]
    InvalidConfiguration(String),
    /// The question violates a bounded input contract.
    #[error("invalid RAG question: {0}")]
    InvalidQuestion(String),
    /// Embedding generation or input guardrails failed.
    #[error("RAG embedding failed: {0}")]
    Embedding(#[source] AiError),
    /// The application retriever failed.
    #[error(transparent)]
    Retrieval(#[from] RagRetrievalError),
    /// A returned document violated the retrieval contract.
    #[error("invalid RAG document: {0}")]
    InvalidDocument(String),
    /// Retrieved content matched the prompt-injection heuristic boundary.
    #[error("RAG context '{document_id}' was rejected: {reason}")]
    UnsafeContext {
        /// Bounded application document identifier.
        document_id: String,
        /// Stable guardrail failure description.
        reason: String,
    },
    /// No safe document fit the configured context budget.
    #[error("RAG retrieval produced no usable context")]
    NoContext,
    /// Text generation or answer validation failed.
    #[error("RAG generation failed: {0}")]
    Generation(#[source] AiError),
    /// Required audit evidence could not be recorded.
    #[error("RAG audit failed: {0}")]
    AuditUnavailable(String),
}

/// Static-dispatch RAG orchestrator with mandatory tenant context and audit sink.
pub struct RagPipeline<R, A> {
    client: AiClient,
    retriever: R,
    audit: A,
    config: RagConfig,
}

impl<R, A> RagPipeline<R, A>
where
    R: RagRetriever,
    A: RagAuditSink,
{
    /// Creates a pipeline with the bounded default context budget.
    pub fn new(client: AiClient, retriever: R, audit: A) -> Self {
        Self {
            client,
            retriever,
            audit,
            config: RagConfig::default(),
        }
    }

    /// Replaces the already-validated context budget.
    pub fn with_config(mut self, config: RagConfig) -> Self {
        self.config = config;
        self
    }

    /// Embeds, retrieves, budgets, generates, and audits one tenant-bound question.
    pub async fn answer(
        &self,
        tenant: &TenantContext,
        question: impl Into<String>,
    ) -> Result<RagAnswer, RagError> {
        let question = question.into();
        let query_sha256 = sha256_hex(question.as_bytes());
        if let Err(error) = validate_question(&question) {
            self.record(
                tenant,
                &query_sha256,
                RagAuditOutcome::QuestionRejected,
                0,
                0,
                0,
            )?;
            return Err(error);
        }

        let embedding = match self.client.embed(&question).await {
            Ok(embedding) if valid_embedding(&embedding) => embedding,
            Ok(_) => {
                self.record(
                    tenant,
                    &query_sha256,
                    RagAuditOutcome::EmbeddingFailed,
                    0,
                    0,
                    0,
                )?;
                return Err(RagError::InvalidQuestion(
                    "embedding must contain finite values within the dimension limit".to_string(),
                ));
            }
            Err(error) => {
                self.record(
                    tenant,
                    &query_sha256,
                    RagAuditOutcome::EmbeddingFailed,
                    0,
                    0,
                    0,
                )?;
                return Err(RagError::Embedding(error));
            }
        };

        let documents = match self
            .retriever
            .retrieve(tenant, &embedding, self.config.max_documents)
            .await
        {
            Ok(documents) => documents,
            Err(error) => {
                self.record(
                    tenant,
                    &query_sha256,
                    RagAuditOutcome::RetrievalFailed,
                    0,
                    0,
                    0,
                )?;
                return Err(RagError::Retrieval(error));
            }
        };
        let retrieved_count = documents.len();
        if retrieved_count > self.config.max_documents {
            self.record(
                tenant,
                &query_sha256,
                RagAuditOutcome::ContextRejected,
                retrieved_count,
                0,
                0,
            )?;
            return Err(RagError::InvalidDocument(format!(
                "retriever returned {retrieved_count} documents, exceeding the requested limit {}",
                self.config.max_documents
            )));
        }

        let (contexts, sources, context_chars) =
            match select_context(tenant, documents, self.config) {
                Ok(selection) => selection,
                Err(error) => {
                    self.record(
                        tenant,
                        &query_sha256,
                        RagAuditOutcome::ContextRejected,
                        retrieved_count,
                        0,
                        0,
                    )?;
                    return Err(error);
                }
            };
        if contexts.is_empty() {
            self.record(
                tenant,
                &query_sha256,
                RagAuditOutcome::NoContext,
                retrieved_count,
                0,
                0,
            )?;
            return Err(RagError::NoContext);
        }

        let prompt = build_rag_prompt(&question, &contexts);
        let answer = match self.client.prompt(&prompt).await {
            Ok(answer) if !answer.trim().is_empty() && answer.len() <= MAX_ANSWER_BYTES => answer,
            Ok(_) => {
                self.record(
                    tenant,
                    &query_sha256,
                    RagAuditOutcome::GenerationFailed,
                    retrieved_count,
                    sources.len(),
                    context_chars,
                )?;
                return Err(RagError::Generation(AiError::ApiError(format!(
                    "provider answer must contain 1-{MAX_ANSWER_BYTES} bytes"
                ))));
            }
            Err(error) => {
                self.record(
                    tenant,
                    &query_sha256,
                    RagAuditOutcome::GenerationFailed,
                    retrieved_count,
                    sources.len(),
                    context_chars,
                )?;
                return Err(RagError::Generation(error));
            }
        };

        self.record(
            tenant,
            &query_sha256,
            RagAuditOutcome::Succeeded,
            retrieved_count,
            sources.len(),
            context_chars,
        )?;
        Ok(RagAnswer { answer, sources })
    }

    fn record(
        &self,
        tenant: &TenantContext,
        query_sha256: &str,
        outcome: RagAuditOutcome,
        retrieved_documents: usize,
        included_documents: usize,
        context_chars: usize,
    ) -> Result<(), RagError> {
        self.audit
            .record(RagAuditEvent {
                tenant_id: tenant.tenant_id.clone(),
                query_sha256: query_sha256.to_string(),
                retrieved_documents,
                included_documents,
                context_chars,
                outcome,
            })
            .map_err(|error| RagError::AuditUnavailable(error.to_string()))
    }
}

fn validate_question(question: &str) -> Result<(), RagError> {
    if question.trim().is_empty() || question.len() > MAX_QUESTION_BYTES {
        return Err(RagError::InvalidQuestion(format!(
            "question must contain 1-{MAX_QUESTION_BYTES} bytes"
        )));
    }
    Ok(())
}

fn valid_embedding(embedding: &[f32]) -> bool {
    !embedding.is_empty()
        && embedding.len() <= MAX_EMBEDDING_DIMENSIONS
        && embedding.iter().all(|value| value.is_finite())
}

fn select_context(
    tenant: &TenantContext,
    documents: Vec<RagDocument>,
    config: RagConfig,
) -> Result<(Vec<String>, Vec<RagSource>, usize), RagError> {
    let mut contexts = Vec::new();
    let mut sources = Vec::new();
    let mut remaining = config.max_context_chars;

    for document in documents {
        if document.tenant_id != tenant.tenant_id {
            return Err(RagError::InvalidDocument(format!(
                "document '{}' belongs to a different tenant",
                document.id
            )));
        }
        if remaining == 0 {
            break;
        }
        let prepared =
            AiGuardrails::prepare(&document.content).map_err(|error| RagError::UnsafeContext {
                document_id: document.id.clone(),
                reason: error.to_string(),
            })?;
        let total_chars = prepared.chars().count();
        let included_chars = total_chars.min(config.max_document_chars).min(remaining);
        if included_chars == 0 {
            continue;
        }
        let context = prepared.chars().take(included_chars).collect::<String>();
        contexts.push(context);
        sources.push(RagSource {
            document_id: document.id,
            score: document.score,
            included_chars,
            truncated: included_chars < total_chars,
        });
        remaining -= included_chars;
    }
    let context_chars = config.max_context_chars - remaining;
    Ok((contexts, sources, context_chars))
}

fn sha256_hex(input: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(input);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}
