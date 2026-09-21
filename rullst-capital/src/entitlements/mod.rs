//! Current, tenant-bound plan authorization, independent of usage quotas.
//!
//! Adapters are trusted server code. A snapshot is an assertion by that adapter,
//! not signed payment evidence. Never construct one from browser input, a checkout
//! redirect, an editable subscription projection or an unverified webhook.
//!
//! ```
//! use rullst_capital::{BillingSubject, entitlements::{
//!     EntitlementGate, EntitlementMode, EntitlementPolicy, EntitlementScope,
//! }};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Values derived from authenticated membership and server configuration.
//! let scope = EntitlementScope::new("tenant_a", BillingSubject::try_new("user", "42")?)?;
//! let gate = EntitlementGate::new(EntitlementPolicy::new(
//!     "reports.billing", ["price_pro"], EntitlementMode::Live, 30,
//! )?);
//! assert_eq!(gate.feature(), "reports.billing");
//! // In the protected async handler: gate.authorize(&scope, &trusted_store).await?;
//! # let _ = scope;
//! # Ok(())
//! # }
//! ```

mod gate;
mod state;

pub use gate::{EntitlementClock, EntitlementGate, EntitlementPolicy, SystemEntitlementClock};
pub use state::{EntitlementMode, EntitlementScope, EntitlementSnapshot, EntitlementStatus};

use std::future::Future;

/// Private-data-free failures. Applications normally expose denial as HTTP 403
/// and configuration, clock or storage failures as HTTP 503.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EntitlementError {
    #[error("invalid entitlement configuration or state")]
    Invalid,
    #[error("entitlement storage is unavailable")]
    Unavailable,
    #[error("entitlement clock is invalid or moved backwards")]
    Clock,
    #[error("the current subscription does not authorize this feature")]
    Denied,
}

/// Authoritative current subscription lookup, invoked on every authorization.
///
/// The host must authenticate the scope, serialize provider reconciliation with
/// revocation, and return only committed state. Stale replicas or snapshots do
/// not satisfy this contract. Use a transaction spanning the gate and the domain
/// write if an application needs stronger atomicity than a read-time decision.
pub trait EntitlementStore: Sync {
    fn current(
        &self,
        scope: &EntitlementScope,
    ) -> impl Future<Output = Result<Option<EntitlementSnapshot>, EntitlementError>> + Send;
}

fn identifier(value: &str, maximum: usize) -> Result<(), EntitlementError> {
    if value.is_empty()
        || value.len() > maximum
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(EntitlementError::Invalid);
    }
    Ok(())
}
