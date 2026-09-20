use super::{EntitlementError, identifier};
use crate::BillingSubject;

/// Authenticated tenant and subscription owner; neither is taken from a form.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntitlementScope {
    pub(super) tenant: String,
    pub(super) subject: BillingSubject,
}

impl EntitlementScope {
    pub fn new(
        tenant: impl Into<String>,
        subject: BillingSubject,
    ) -> Result<Self, EntitlementError> {
        let tenant = tenant.into();
        identifier(&tenant, 128)?;
        Ok(Self { tenant, subject })
    }

    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    pub fn subject(&self) -> &BillingSubject {
        &self.subject
    }
}

impl std::fmt::Debug for EntitlementScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EntitlementScope([REDACTED])")
    }
}

/// An explicit policy mode. Sandbox state never satisfies a live policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EntitlementMode {
    Live,
    Sandbox,
    /// Kept for deterministic offline adapters; always denied by the gate.
    Mock,
}

/// Provider status mapped by the trusted adapter; only `Active` authorizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EntitlementStatus {
    Active,
    Trial,
    PastDue,
    Revoked,
    Other,
}

/// Bounded committed subscription state asserted by a trusted server adapter.
///
/// `observed_at` is the trusted time immediately BEFORE the provider read, so a
/// slow response cannot rejuvenate old evidence. `valid_until` is the exclusive
/// subscription-period end, not an arbitrary cache TTL. Neither implies invoice
/// settlement. This value intentionally has no serialization implementation.
#[derive(Clone)]
#[non_exhaustive]
pub struct EntitlementSnapshot {
    pub(super) scope: EntitlementScope,
    pub(super) plan: String,
    pub(super) status: EntitlementStatus,
    pub(super) mode: EntitlementMode,
    pub(super) observed_at: i64,
    pub(super) valid_until: i64,
}

impl EntitlementSnapshot {
    pub fn from_reconciled(
        scope: EntitlementScope,
        plan: impl Into<String>,
        status: EntitlementStatus,
        mode: EntitlementMode,
        observed_at: i64,
        valid_until: i64,
    ) -> Result<Self, EntitlementError> {
        let plan = plan.into();
        identifier(&plan, 200)?;
        if observed_at < 0 || valid_until <= 0 {
            return Err(EntitlementError::Invalid);
        }
        Ok(Self {
            scope,
            plan,
            status,
            mode,
            observed_at,
            valid_until,
        })
    }
}

impl std::fmt::Debug for EntitlementSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntitlementSnapshot")
            .field("status", &self.status)
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}
