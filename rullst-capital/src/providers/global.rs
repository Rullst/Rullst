//! Process-wide default billing and payout providers.

use super::{BillingProvider, PayoutProvider};
use tokio::sync::OnceCell;

static BILLING_PROVIDER: OnceCell<Box<dyn BillingProvider>> = OnceCell::const_new();
static PAYOUT_PROVIDER: OnceCell<Box<dyn PayoutProvider>> = OnceCell::const_new();

/// Initializes the global billing provider.
pub fn init_provider(provider: Box<dyn BillingProvider>) {
    let _ = BILLING_PROVIDER.set(provider);
}

/// Retrieves the active billing provider, or `None` if not initialized.
pub fn provider() -> Option<&'static dyn BillingProvider> {
    BILLING_PROVIDER.get().map(|p| p.as_ref())
}

/// Initializes the global payout provider.
pub fn init_payout_provider(provider: Box<dyn PayoutProvider>) {
    let _ = PAYOUT_PROVIDER.set(provider);
}

/// Retrieves the active payout provider, or `None` if not initialized.
pub fn payout_provider() -> Option<&'static dyn PayoutProvider> {
    PAYOUT_PROVIDER.get().map(|p| p.as_ref())
}
