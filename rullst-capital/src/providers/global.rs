//! Process-wide default billing and payout providers.
//!
//! Each registry can be set once per process. Prefer passing an explicit
//! provider (for example `WebhookMiddlewareState::production_with_provider`)
//! when an application needs more than one configuration.

use super::{BillingProvider, PayoutProvider};
use crate::error::CapitalError;
use tokio::sync::OnceCell;

static BILLING_PROVIDER: OnceCell<Box<dyn BillingProvider>> = OnceCell::const_new();
static PAYOUT_PROVIDER: OnceCell<Box<dyn PayoutProvider>> = OnceCell::const_new();

/// Initializes the global billing provider once.
///
/// A later call is ignored and the first provider stays active; use
/// [`try_init_provider`] to detect that case.
pub fn init_provider(provider: Box<dyn BillingProvider>) {
    let _ = try_init_provider(provider);
}

/// Initializes the global billing provider, or reports that one is already set.
///
/// Returns `ConfigurationError` and keeps the existing provider when the
/// registry was already initialized. New in 13.0.
pub fn try_init_provider(provider: Box<dyn BillingProvider>) -> Result<(), CapitalError> {
    BILLING_PROVIDER
        .set(provider)
        .map_err(|_| already_initialized("billing"))
}

/// Retrieves the active billing provider, or `None` if not initialized.
pub fn provider() -> Option<&'static dyn BillingProvider> {
    BILLING_PROVIDER.get().map(|p| p.as_ref())
}

/// Initializes the global payout provider once.
///
/// A later call is ignored and the first provider stays active; use
/// [`try_init_payout_provider`] to detect that case.
pub fn init_payout_provider(provider: Box<dyn PayoutProvider>) {
    let _ = try_init_payout_provider(provider);
}

/// Initializes the global payout provider, or reports that one is already set.
///
/// Returns `ConfigurationError` and keeps the existing provider when the
/// registry was already initialized. New in 13.0.
pub fn try_init_payout_provider(provider: Box<dyn PayoutProvider>) -> Result<(), CapitalError> {
    PAYOUT_PROVIDER
        .set(provider)
        .map_err(|_| already_initialized("payout"))
}

/// Retrieves the active payout provider, or `None` if not initialized.
pub fn payout_provider() -> Option<&'static dyn PayoutProvider> {
    PAYOUT_PROVIDER.get().map(|p| p.as_ref())
}

fn already_initialized(registry: &str) -> CapitalError {
    CapitalError::ConfigurationError(format!(
        "the global {registry} provider is already initialized; it can be set once per process"
    ))
}
