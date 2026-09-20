use super::{ConsentError, valid_ref};
use std::fmt;

/// Opaque subject/tenant references resolved from authenticated server state.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConsentSubject {
    subject: String,
    tenant: String,
}

impl ConsentSubject {
    pub fn new(
        subject: impl Into<String>,
        tenant: impl Into<String>,
    ) -> Result<Self, ConsentError> {
        let value = Self {
            subject: subject.into(),
            tenant: tenant.into(),
        };
        if !valid_ref(&value.subject) || !valid_ref(&value.tenant) {
            return Err(ConsentError::InvalidConfiguration);
        }
        Ok(value)
    }

    pub fn subject_ref(&self) -> &str {
        &self.subject
    }
    pub fn tenant_ref(&self) -> &str {
        &self.tenant
    }
}

impl fmt::Debug for ConsentSubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConsentSubject([redacted])")
    }
}

/// Server-selected optional purpose and the exact notice version shown to users.
/// Change the version when processing changes; never reuse a retired version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentPurpose {
    id: String,
    version: String,
}

impl ConsentPurpose {
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Result<Self, ConsentError> {
        let value = Self {
            id: id.into(),
            version: version.into(),
        };
        if !valid_ref(&value.id) || !valid_ref(&value.version) {
            return Err(ConsentError::InvalidConfiguration);
        }
        Ok(value)
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentChoice {
    Unset,
    Granted,
    Declined,
    Withdrawn,
}

/// Explicit form/API choice. Its displayed purpose/version and revision must
/// match the server's current policy and durable state before a grant is saved.
#[derive(Debug, Clone)]
pub struct ConsentSubmission {
    pub(super) displayed: ConsentPurpose,
    pub(super) revision: u64,
    pub(super) choice: ConsentChoice,
}

impl ConsentSubmission {
    pub fn new(
        displayed: ConsentPurpose,
        revision: u64,
        choice: ConsentChoice,
    ) -> Result<Self, ConsentError> {
        if !matches!(choice, ConsentChoice::Granted | ConsentChoice::Declined)
            || revision > i64::MAX as u64
        {
            return Err(ConsentError::InvalidConfiguration);
        }
        Ok(Self {
            displayed,
            revision,
            choice,
        })
    }
}

/// Latest state, not a bearer permission or reusable authorization receipt.
/// Only a fresh gate check can allow the current purpose and context.
#[derive(Clone, PartialEq, Eq)]
pub struct ConsentRecord {
    subject: ConsentSubject,
    purpose_id: String,
    revision: u64,
    version: String,
    choice: ConsentChoice,
    changed_at: i64,
    valid_until: i64,
}

impl fmt::Debug for ConsentRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConsentRecord([redacted])")
    }
}

impl ConsentRecord {
    /// Trusted adapters use this only when the authoritative key is absent.
    pub fn absent(
        subject: ConsentSubject,
        purpose_id: impl Into<String>,
    ) -> Result<Self, ConsentError> {
        let purpose_id = purpose_id.into();
        if !valid_ref(&purpose_id) {
            return Err(ConsentError::InvalidConfiguration);
        }
        Ok(Self {
            subject,
            purpose_id,
            revision: 0,
            version: String::new(),
            choice: ConsentChoice::Unset,
            changed_at: 0,
            valid_until: 0,
        })
    }

    /// Restore trusted durable storage, never a client-supplied consent record.
    /// The host is responsible for storage integrity and authenticated lookup.
    pub fn from_stored(
        subject: ConsentSubject,
        purpose: ConsentPurpose,
        revision: u64,
        choice: ConsentChoice,
        changed_at: i64,
        valid_until: i64,
    ) -> Result<Self, ConsentError> {
        if revision == 0
            || revision > i64::MAX as u64
            || changed_at < 0
            || choice == ConsentChoice::Unset
            || !valid_expiry(choice, changed_at, valid_until)
        {
            return Err(ConsentError::StoreConfiguration);
        }
        Ok(Self {
            subject,
            purpose_id: purpose.id,
            revision,
            version: purpose.version,
            choice,
            changed_at,
            valid_until,
        })
    }

    pub fn subject(&self) -> &ConsentSubject {
        &self.subject
    }
    pub fn purpose_id(&self) -> &str {
        &self.purpose_id
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn choice(&self) -> ConsentChoice {
        self.choice
    }
    pub fn changed_at(&self) -> i64 {
        self.changed_at
    }
    pub fn valid_until(&self) -> i64 {
        self.valid_until
    }

    pub(super) fn validate_scope(
        &self,
        subject: &ConsentSubject,
        purpose: &ConsentPurpose,
        now: i64,
    ) -> Result<(), ConsentError> {
        if &self.subject != subject || self.purpose_id != purpose.id {
            return Err(ConsentError::BindingMismatch);
        }
        if now < 0 || now < self.changed_at {
            return Err(ConsentError::ClockRollback);
        }
        Ok(())
    }
}

/// Validated server operation. Adapters must apply it inside their exclusive
/// state transaction; a stale positive form never overrides a withdrawal.
#[derive(Clone)]
pub struct ConsentUpdate {
    pub(super) subject: ConsentSubject,
    pub(super) purpose: ConsentPurpose,
    pub(super) expected_revision: Option<u64>,
    pub(super) choice: ConsentChoice,
    pub(super) now: i64,
    pub(super) valid_until: i64,
}

impl fmt::Debug for ConsentUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConsentUpdate([redacted])")
    }
}

impl ConsentUpdate {
    pub fn subject(&self) -> &ConsentSubject {
        &self.subject
    }
    pub fn purpose(&self) -> &ConsentPurpose {
        &self.purpose
    }
    pub fn now(&self) -> i64 {
        self.now
    }

    pub fn apply_to(&self, current: &ConsentRecord) -> Result<ConsentRecord, ConsentError> {
        current.validate_scope(&self.subject, &self.purpose, self.now)?;
        if self
            .expected_revision
            .is_some_and(|expected| expected != current.revision)
        {
            return Err(ConsentError::RevisionConflict);
        }
        let revision = current
            .revision
            .checked_add(1)
            .filter(|v| *v <= i64::MAX as u64)
            .ok_or(ConsentError::RevisionExhausted)?;
        if revision == i64::MAX as u64 && self.choice == ConsentChoice::Granted {
            return Err(ConsentError::RevisionExhausted);
        }
        ConsentRecord::from_stored(
            self.subject.clone(),
            self.purpose.clone(),
            revision,
            self.choice,
            self.now,
            self.valid_until,
        )
    }
}

pub(super) fn valid_expiry(choice: ConsentChoice, now: i64, expiry: i64) -> bool {
    if choice == ConsentChoice::Granted {
        // A bounded engineering lifetime, never a jurisdictional legal default.
        expiry
            .checked_sub(now)
            .is_some_and(|remaining| (1..=31_536_000).contains(&remaining))
    } else {
        expiry == 0
    }
}
