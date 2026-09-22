use crate::PublishReceipt;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OccurrenceState {
    Pending,
    Leased,
    Published,
    DeadLetter,
    Cancelled,
}

/// Metadata only: payload, headers, keys and lease credentials are excluded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccurrenceMetadata {
    pub(super) id: String,
    pub(super) schedule: String,
    pub(super) due: i64,
    pub(super) created: i64,
    pub(super) expires: i64,
    pub(super) attempts: u32,
    pub(super) state: OccurrenceState,
}
impl OccurrenceMetadata {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn schedule_name(&self) -> &str {
        &self.schedule
    }
    pub fn due_at_ms(&self) -> i64 {
        self.due
    }
    pub fn created_at_ms(&self) -> i64 {
        self.created
    }
    pub fn expires_at_ms(&self) -> i64 {
        self.expires
    }
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
    pub fn state(&self) -> OccurrenceState {
        self.state
    }
}

/// Single fenced dispatch capability. It cannot be constructed from request data.
pub struct OccurrenceLease {
    pub(super) namespace: String,
    pub(super) token: Zeroizing<String>,
    pub(super) version: i64,
    pub(super) expires: i64,
    pub(super) metadata: OccurrenceMetadata,
}
impl OccurrenceLease {
    pub fn metadata(&self) -> &OccurrenceMetadata {
        &self.metadata
    }
    pub fn lease_expires_at_ms(&self) -> i64 {
        self.expires
    }
}
impl std::fmt::Debug for OccurrenceLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OccurrenceLease([REDACTED])")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecurringMetadata {
    pub(super) name: String,
    pub(super) generation: String,
    pub(super) created: i64,
    pub(super) next_due: Option<i64>,
    pub(super) cancelled: bool,
}
impl RecurringMetadata {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn generation(&self) -> &str {
        &self.generation
    }
    pub fn created_at_ms(&self) -> i64 {
        self.created
    }
    pub fn next_due_at_ms(&self) -> Option<i64> {
        self.next_due
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecurringRelayReceipt {
    pub(super) publication: PublishReceipt,
    pub(super) acknowledged: bool,
}
impl RecurringRelayReceipt {
    pub fn publication(&self) -> &PublishReceipt {
        &self.publication
    }
    pub fn occurrence_acknowledged(&self) -> bool {
        self.acknowledged
    }
}
