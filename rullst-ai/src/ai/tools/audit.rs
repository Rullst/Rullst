use super::{ToolExecutionError, ToolRisk};
use crate::ai::durable_audit::{
    DurableAuditError, DurableAuditLog, DurableAuditRecord, DurableAuditSnapshot,
};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, path::PathBuf, sync::Mutex};

const TOOL_AUDIT_MAGIC: &[u8] = b"RULLST-AI-TOOL-AUDIT-V1\n";

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

/// Sequence-numbered event returned by the in-memory and durable tool trails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordedToolAuditEvent {
    pub sequence: u64,
    pub event: ToolAuditEvent,
}

impl DurableAuditRecord for ToolAuditEvent {
    const MAGIC: &'static [u8] = TOOL_AUDIT_MAGIC;

    fn validate(&self) -> Result<(), &'static str> {
        if self.principal.trim().is_empty() || self.principal.len() > 256 {
            return Err("event has an invalid principal");
        }
        if self.tool.is_empty()
            || self.tool.len() > 64
            || !self
                .tool
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err("event has an invalid tool identifier");
        }
        match (&self.approved_by, &self.approval_reason) {
            (None, None) => {}
            (Some(approver), Some(reason))
                if !approver.trim().is_empty()
                    && approver.len() <= 256
                    && !reason.trim().is_empty()
                    && reason.len() <= 2 * 1024 => {}
            _ => return Err("event has invalid approval evidence"),
        }
        if self.risk.is_none() && self.outcome != ToolAuditOutcome::Denied {
            return Err("event omits risk for a non-denied outcome");
        }
        Ok(())
    }
}

/// Bounded single-process durable tool audit trail.
///
/// The file is synchronously appended and validated on restart. It is not a
/// multi-process writer, external SIEM, retention service, or authenticity
/// proof; the host owns directory permissions, rotation, backup, and delivery.
pub struct DurableToolAuditTrail {
    log: DurableAuditLog<ToolAuditEvent>,
}

impl DurableToolAuditTrail {
    /// Opens or creates a local tool audit file with the crate's 16 MiB ceiling.
    pub fn try_open(path: impl Into<PathBuf>) -> Result<Self, DurableAuditError> {
        DurableAuditLog::try_open(path).map(|log| Self { log })
    }

    /// Opens or creates a local tool audit file with a smaller explicit quota.
    pub fn try_open_with_max_bytes(
        path: impl Into<PathBuf>,
        max_bytes: u64,
    ) -> Result<Self, DurableAuditError> {
        DurableAuditLog::try_open_with_max_bytes(path, max_bytes).map(|log| Self { log })
    }

    /// Re-reads and validates all durable entries in sequence order.
    pub fn entries(&self) -> Result<Vec<RecordedToolAuditEvent>, DurableAuditError> {
        self.log
            .entries()?
            .into_iter()
            .enumerate()
            .map(|(index, event)| {
                let sequence = u64::try_from(index)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .ok_or(DurableAuditError::RecordCapacityExceeded)?;
                Ok(RecordedToolAuditEvent { sequence, event })
            })
            .collect()
    }

    /// Returns validated counters without exposing event bodies.
    pub fn snapshot(&self) -> Result<DurableAuditSnapshot, DurableAuditError> {
        self.log.snapshot()
    }
}

impl ToolAuditSink for DurableToolAuditTrail {
    fn record(&self, event: ToolAuditEvent) -> Result<(), ToolExecutionError> {
        self.log
            .append(event)
            .map_err(|error| ToolExecutionError::AuditUnavailable(error.to_string()))
    }
}

#[derive(Debug, Default)]
struct InMemoryAuditState {
    next_sequence: u64,
    events: VecDeque<RecordedToolAuditEvent>,
}

/// Bounded process-local audit sink for development and single-process tests.
///
/// Single-process services can use [`DurableToolAuditTrail`]. Multi-instance
/// deployments should implement an append-only shared sink.
#[derive(Debug)]
pub struct InMemoryToolAuditTrail {
    capacity: usize,
    state: Mutex<InMemoryAuditState>,
}

impl InMemoryToolAuditTrail {
    pub fn new(capacity: usize) -> Result<Self, ToolExecutionError> {
        if !(1..=1_000_000).contains(&capacity) {
            return Err(ToolExecutionError::InvalidPolicy(
                "tool audit capacity must be between 1 and 1000000".to_string(),
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
