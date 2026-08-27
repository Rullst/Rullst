use super::{ToolExecutionError, ToolRisk};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Mutex};

/// Result classification stored by a [`ToolAuditSink`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolAuditOutcome {
    Denied,
    Authorized,
    Succeeded,
    Failed,
}

/// Secret-free audit event for one dispatch decision or outcome.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolAuditEvent {
    pub principal: String,
    pub tool: String,
    pub risk: Option<ToolRisk>,
    /// Authenticated approver identity supplied by the application, when required.
    pub approved_by: Option<String>,
    /// Bounded application-supplied reason associated with the approval.
    pub approval_reason: Option<String>,
    pub outcome: ToolAuditOutcome,
}

/// Application-provided audit destination. An unavailable sink fails execution closed.
pub trait ToolAuditSink: Send + Sync {
    fn record(&self, event: ToolAuditEvent) -> Result<(), ToolExecutionError>;
}

/// Sequence-numbered event stored by [`InMemoryToolAuditTrail`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordedToolAuditEvent {
    pub sequence: u64,
    pub event: ToolAuditEvent,
}

#[derive(Debug, Default)]
struct InMemoryAuditState {
    next_sequence: u64,
    events: VecDeque<RecordedToolAuditEvent>,
}

/// Bounded process-local audit sink for development and single-process tests.
/// Production multi-instance deployments should implement a durable append-only sink.
#[derive(Debug)]
pub struct InMemoryToolAuditTrail {
    capacity: usize,
    state: Mutex<InMemoryAuditState>,
}

impl InMemoryToolAuditTrail {
    pub fn new(capacity: usize) -> Result<Self, ToolExecutionError> {
        if capacity == 0 {
            return Err(ToolExecutionError::InvalidPolicy(
                "tool audit capacity must be greater than zero".to_string(),
            ));
        }
        Ok(Self {
            capacity,
            state: Mutex::new(InMemoryAuditState::default()),
        })
    }

    pub fn entries(&self) -> Result<Vec<RecordedToolAuditEvent>, ToolExecutionError> {
        self.state
            .lock()
            .map(|state| state.events.iter().cloned().collect())
            .map_err(|_| {
                ToolExecutionError::AuditUnavailable("audit lock was poisoned".to_string())
            })
    }
}

impl ToolAuditSink for InMemoryToolAuditTrail {
    fn record(&self, event: ToolAuditEvent) -> Result<(), ToolExecutionError> {
        let mut state = self.state.lock().map_err(|_| {
            ToolExecutionError::AuditUnavailable("audit lock was poisoned".to_string())
        })?;
        let sequence = state.next_sequence.checked_add(1).ok_or_else(|| {
            ToolExecutionError::AuditUnavailable("audit sequence exhausted".to_string())
        })?;
        state.next_sequence = sequence;
        if state.events.len() == self.capacity {
            state.events.pop_front();
        }
        state
            .events
            .push_back(RecordedToolAuditEvent { sequence, event });
        Ok(())
    }
}
