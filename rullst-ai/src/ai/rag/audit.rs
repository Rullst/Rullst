//! Secret-minimized audit contract for bounded RAG operations.

use crate::ai::durable_audit::{
    DurableAuditError, DurableAuditLog, DurableAuditRecord, DurableAuditSnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const RAG_AUDIT_MAGIC: &[u8] = b"RULLST-AI-RAG-AUDIT-V1\n";

/// Final outcome recorded for one RAG operation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RagAuditOutcome {
    /// The question was empty or exceeded the bounded input contract.
    QuestionRejected,
    /// Embedding or input guardrails failed.
    EmbeddingFailed,
    /// The application-provided retriever failed.
    RetrievalFailed,
    /// A retrieved document violated a tenant, size, score, or guardrail boundary.
    ContextRejected,
    /// Retrieval produced no context that fit the configured budget.
    NoContext,
    /// Model generation or response validation failed.
    GenerationFailed,
    /// Retrieval, generation, validation, and audit all succeeded.
    Succeeded,
}

/// Secret-minimized evidence for one RAG attempt.
///
/// Question text, context, embeddings, model output, and provider error bodies are deliberately
/// absent. `query_sha256` is a correlation digest, not encryption, and can still be susceptible to
/// guessing for low-entropy questions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RagAuditEvent {
    /// Tenant selected by trusted authentication context.
    pub tenant_id: String,
    /// Lowercase SHA-256 digest of the original question.
    pub query_sha256: String,
    /// Number of documents returned by the retriever.
    pub retrieved_documents: usize,
    /// Number of documents included within the context budget.
    pub included_documents: usize,
    /// Unicode scalar count included in the generated prompt context.
    pub context_chars: usize,
    /// Bounded operation outcome without provider or document content.
    pub outcome: RagAuditOutcome,
}

/// Failure returned by an application-provided RAG audit sink.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("RAG audit unavailable: {0}")]
pub struct RagAuditError(pub String);

/// Application-provided RAG audit destination. An unavailable sink fails the operation closed.
pub trait RagAuditSink: Send + Sync {
    /// Records one terminal event without raw question, context, embedding, or answer data.
    fn record(&self, event: RagAuditEvent) -> Result<(), RagAuditError>;
}

impl<T> RagAuditSink for Arc<T>
where
    T: RagAuditSink + ?Sized,
{
    fn record(&self, event: RagAuditEvent) -> Result<(), RagAuditError> {
        (**self).record(event)
    }
}

/// Sequence-numbered event returned by the in-memory and durable RAG trails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordedRagAuditEvent {
    /// Monotonic sequence within this trail.
    pub sequence: u64,
    /// Recorded secret-minimized event.
    pub event: RagAuditEvent,
}

impl DurableAuditRecord for RagAuditEvent {
    const MAGIC: &'static [u8] = RAG_AUDIT_MAGIC;

    fn validate(&self) -> Result<(), &'static str> {
        if self.tenant_id.is_empty()
            || self.tenant_id.len() > 128
            || !self.tenant_id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
        {
            return Err("event has an invalid tenant identifier");
        }
        if self.query_sha256.len() != 64
            || !self
                .query_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err("event has an invalid query digest");
        }
        if self.included_documents > 32 || self.included_documents > self.retrieved_documents {
            return Err("event has invalid document counts");
        }
        if self.context_chars > 128 * 1024 {
            return Err("event exceeds the context character limit");
        }
        Ok(())
    }
}

/// Bounded single-process durable RAG audit trail.
///
/// The file is synchronously appended and validated on restart. It is not a
/// multi-process writer, external SIEM, retention service, or authenticity
/// proof; the host owns directory permissions, rotation, backup, and delivery.
pub struct DurableRagAuditTrail {
    log: DurableAuditLog<RagAuditEvent>,
}

impl DurableRagAuditTrail {
    /// Opens or creates a local RAG audit file with the crate's 16 MiB ceiling.
    pub fn try_open(path: impl Into<PathBuf>) -> Result<Self, DurableAuditError> {
        DurableAuditLog::try_open(path).map(|log| Self { log })
    }

    /// Opens or creates a local RAG audit file with a smaller explicit quota.
    pub fn try_open_with_max_bytes(
        path: impl Into<PathBuf>,
        max_bytes: u64,
    ) -> Result<Self, DurableAuditError> {
        DurableAuditLog::try_open_with_max_bytes(path, max_bytes).map(|log| Self { log })
    }

    /// Re-reads and validates all durable entries in sequence order.
    pub fn entries(&self) -> Result<Vec<RecordedRagAuditEvent>, DurableAuditError> {
        self.log
            .entries()?
            .into_iter()
            .enumerate()
            .map(|(index, event)| {
                let sequence = u64::try_from(index)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .ok_or(DurableAuditError::RecordCapacityExceeded)?;
                Ok(RecordedRagAuditEvent { sequence, event })
            })
            .collect()
    }

    /// Returns validated counters without exposing event bodies.
    pub fn snapshot(&self) -> Result<DurableAuditSnapshot, DurableAuditError> {
        self.log.snapshot()
    }
}

impl RagAuditSink for DurableRagAuditTrail {
    fn record(&self, event: RagAuditEvent) -> Result<(), RagAuditError> {
        self.log
            .append(event)
            .map_err(|error| RagAuditError(error.to_string()))
    }
}

#[derive(Debug, Default)]
struct InMemoryRagAuditState {
    next_sequence: u64,
    events: VecDeque<RecordedRagAuditEvent>,
}

/// Bounded process-local RAG audit trail for development and tests.
///
/// Single-process services can use [`DurableRagAuditTrail`]. Multi-instance
/// deployments should provide an append-only shared implementation.
#[derive(Debug)]
pub struct InMemoryRagAuditTrail {
    capacity: usize,
    state: Mutex<InMemoryRagAuditState>,
}

impl InMemoryRagAuditTrail {
    /// Creates a bounded audit trail.
    pub fn new(capacity: usize) -> Result<Self, RagAuditError> {
        if capacity == 0 || capacity > 1_000_000 {
            return Err(RagAuditError(
                "in-memory RAG audit capacity must be between 1 and 1,000,000".to_string(),
            ));
        }
        Ok(Self {
            capacity,
            state: Mutex::new(InMemoryRagAuditState::default()),
        })
    }

    /// Returns a stable snapshot of retained entries.
    pub fn entries(&self) -> Result<Vec<RecordedRagAuditEvent>, RagAuditError> {
        self.state
            .lock()
            .map(|state| state.events.iter().cloned().collect())
            .map_err(|_| RagAuditError("in-memory RAG audit lock was poisoned".to_string()))
    }
}

impl RagAuditSink for InMemoryRagAuditTrail {
    fn record(&self, event: RagAuditEvent) -> Result<(), RagAuditError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RagAuditError("in-memory RAG audit lock was poisoned".to_string()))?;
        let sequence = state
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| RagAuditError("in-memory RAG audit sequence exhausted".to_string()))?;
        state.next_sequence = sequence;
        if state.events.len() == self.capacity {
            state.events.pop_front();
        }
        state
            .events
            .push_back(RecordedRagAuditEvent { sequence, event });
        Ok(())
    }
}
