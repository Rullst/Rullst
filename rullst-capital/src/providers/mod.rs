use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use subtle::ConstantTimeEq;

mod fixture;
mod global;
mod http;
pub mod infinitepay;
mod request;
pub mod stripe;
mod stripe_charge;
mod stripe_checkout;
mod stripe_one_off;
mod stripe_one_time;
mod stripe_reconcile;
pub use stripe_reconcile::StripeCheckoutSnapshot;
pub(crate) mod stripe_contract;
mod stripe_customer;
mod stripe_event;
mod stripe_snapshot;
mod stripe_subscription;
mod stripe_usage;
mod stripe_webhook;

pub use global::{init_provider, provider, try_init_provider};
pub(crate) use http::{execute as execute_http, read_json as read_http_json};
pub(crate) use http::{require_https_redirects, validate_checkout_url, validate_redirect_url};
pub(crate) use http::{send as send_http, send_json as send_http_json};
pub use infinitepay::InfinitePayProvider;
pub use request::ProviderRequest;
pub use stripe::StripeProvider;

/// Maximum clock drift accepted by timestamped webhook protocols by default.
pub const DEFAULT_WEBHOOK_TOLERANCE: Duration = Duration::from_secs(5 * 60);

// Compatibility-only operations may retain deterministic offline fixtures,
// but live credentials must never return a fabricated session or no-op success.
pub(crate) fn require_mock_operation(
    api_key: &str,
    provider: &str,
    operation: &str,
) -> Result<(), crate::CapitalError> {
    if api_key.is_empty() || api_key.starts_with("mock_") {
        return Ok(());
    }
    Err(crate::CapitalError::UnsupportedOperation(format!(
        "{provider} {operation} has no reviewed live provider contract"
    )))
}

/// How a provider authenticates its webhook notifications.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookVerificationMode {
    /// Provider-native cryptographic verification with a non-empty real secret.
    Real,
    /// Deterministic local-only verification selected by a `mock_*` secret.
    Mock,
    /// The notification is unauthenticated: it only names a payment, and the
    /// payment is confirmed by querying the provider.
    ///
    /// The webhook middleware never calls [`BillingProvider::handle_webhook`]
    /// for such a provider and never inserts a [`WebhookEvent`]: it reads the
    /// body with [`BillingProvider::parse_payment_notification`] and inserts
    /// the resulting [`PaymentNotification`], which carries no payment status.
    /// Only [`BillingProvider::confirm_one_off_payment`], which queries the
    /// provider and compares the paid amount with the server-side request,
    /// can report a payment as paid. Since 13.0.0-alpha.5.
    ConfirmedAtSource,
}

pub(crate) fn webhook_mode_from_secret(
    provider: &str,
    secret: &str,
) -> Result<WebhookVerificationMode, crate::error::CapitalError> {
    let secret = secret.trim();
    if secret.is_empty() {
        return Err(crate::error::CapitalError::ConfigurationError(format!(
            "{provider} webhook secret cannot be empty"
        )));
    }

    if secret.starts_with("mock_") {
        Ok(WebhookVerificationMode::Mock)
    } else {
        Ok(WebhookVerificationMode::Real)
    }
}

pub(crate) fn verify_explicit_mock_signature(
    provider: &str,
    secret: &str,
    signature: &str,
) -> Result<(), crate::error::CapitalError> {
    if secret.as_bytes().ct_eq(signature.as_bytes()).unwrap_u8() == 1 {
        Ok(())
    } else {
        Err(crate::error::CapitalError::InvalidSignature(format!(
            "{provider} mock signature verification failed"
        )))
    }
}

pub(crate) fn ensure_fresh_timestamp(
    provider: &str,
    timestamp: &str,
    now_unix_seconds: i64,
    tolerance: Duration,
    allow_milliseconds: bool,
) -> Result<(), crate::error::CapitalError> {
    let raw_timestamp = timestamp.parse::<i64>().map_err(|_| {
        crate::error::CapitalError::InvalidSignature(format!(
            "Invalid {provider} webhook timestamp"
        ))
    })?;
    let timestamp_seconds = if allow_milliseconds && raw_timestamp.unsigned_abs() >= 100_000_000_000
    {
        raw_timestamp / 1_000
    } else {
        raw_timestamp
    };
    let tolerance_seconds = i64::try_from(tolerance.as_secs()).unwrap_or(i64::MAX);
    let lower_bound = now_unix_seconds.saturating_sub(tolerance_seconds);
    let upper_bound = now_unix_seconds.saturating_add(tolerance_seconds);

    if !(lower_bound..=upper_bound).contains(&timestamp_seconds) {
        return Err(crate::error::CapitalError::StaleWebhook(format!(
            "{provider} timestamp {timestamp_seconds} is outside the {tolerance_seconds}s acceptance window"
        )));
    }

    Ok(())
}

/// Returns the fail-closed shared client used by reviewed live provider adapters.
pub(crate) fn http_client() -> Result<&'static reqwest::Client, crate::error::CapitalError> {
    http::client()
}

/// The semantic status of a SaaS Subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// The subscription is active and in good standing.
    Active,
    /// The subscription was canceled.
    Canceled,
    /// The subscription is past due but not yet unpaid.
    PastDue,
    /// The subscription is unpaid and access is revoked.
    Unpaid,
    /// The subscription is currently in a free trial period.
    Trialing,
    /// The subscription has been paused.
    Paused,
}

impl SubscriptionStatus {
    /// Returns the static string representation of the subscription status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Canceled => "canceled",
            Self::PastDue => "past_due",
            Self::Unpaid => "unpaid",
            Self::Trialing => "trialing",
            Self::Paused => "paused",
        }
    }

    /// Parses a string representation of a subscription status.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn parse_status(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" | "paid" | "completed" | "approved" | "resolved" => Self::Active,
            "canceled" | "cancelled" => Self::Canceled,
            "past_due" => Self::PastDue,
            "unpaid" | "failed" | "rejected" | "expired" => Self::Unpaid,
            "trialing" => Self::Trialing,
            "paused" => Self::Paused,
            _ => Self::Unpaid,
        }
    }
}

/// Unified model representing a webhook event for subscription/payment changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Unique identifier of the subscription or charge in the provider system.
    pub subscription_id: String,
    /// Unique customer ID in the provider system.
    pub customer_id: String,
    /// Email of the customer.
    pub customer_email: String,
    /// The ID of the plan / product / price.
    pub plan_id: String,
    /// The status of the subscription.
    pub status: SubscriptionStatus,
    /// Expiration / end date timestamp (if applicable).
    pub ends_at: Option<i64>,
}

use crate::error::CapitalError;
use crate::{
    ChargeReceipt, ChargeRequest, OneOffCheckout, OneOffCheckoutRequest, OneOffPayment,
    PaymentNotification,
};

/// Dynamic trait to handle billing provider interactions.
#[async_trait]
pub trait BillingProvider: Send + Sync {
    /// Return the name of the billing provider (e.g. "stripe" or "infinitepay").
    fn name(&self) -> &'static str;

    /// Returns the explicitly selected webhook verification mode.
    ///
    /// The fail-closed default preserves source compatibility for custom providers while requiring
    /// them to opt in before they can be mounted behind Rullst's webhook middleware.
    fn webhook_verification_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        Err(CapitalError::ConfigurationError(format!(
            "{} must explicitly declare its webhook verification mode",
            self.name()
        )))
    }

    /// Create a checkout session URL for a customer.
    async fn create_checkout_session(
        &self,
        customer_email: &str,
        plan_id: &str,
        redirect_url: &str,
    ) -> Result<String, CapitalError>;

    /// Attempts one fully specified immediate off-session charge.
    ///
    /// Adapters must override this method only after forwarding the request's
    /// idempotency key and binding the provider response to its amount and
    /// currency. The default prevents adapter presence from implying support.
    async fn charge(&self, _request: &ChargeRequest) -> Result<ChargeReceipt, CapitalError> {
        Err(CapitalError::UnsupportedOperation(format!(
            "{} does not provide reviewed direct-charge support",
            self.name()
        )))
    }

    /// Creates a hosted checkout for one purchase priced on the server.
    ///
    /// The amount, currency, description and quantity come from the
    /// [`OneOffCheckoutRequest`], never from the browser. Creating the
    /// checkout proves nothing: confirm the payment with
    /// [`Self::confirm_one_off_payment`] or the provider's signed events. The
    /// default returns [`CapitalError::UnsupportedOperation`]. Since
    /// 13.0.0-alpha.5.
    async fn create_one_off_checkout(
        &self,
        _request: &OneOffCheckoutRequest,
    ) -> Result<OneOffCheckout, CapitalError> {
        Err(CapitalError::UnsupportedOperation(format!(
            "{} does not provide reviewed one-off checkout support",
            self.name()
        )))
    }

    /// Reads which payment an unauthenticated notification names.
    ///
    /// Providers in [`WebhookVerificationMode::ConfirmedAtSource`] implement
    /// it; the webhook middleware calls it instead of
    /// [`Self::handle_webhook`]. The result carries no payment status. The
    /// default returns [`CapitalError::UnsupportedOperation`]. Since
    /// 13.0.0-alpha.5.
    fn parse_payment_notification(
        &self,
        _payload: &[u8],
        _headers: &HashMap<String, String>,
    ) -> Result<PaymentNotification, CapitalError> {
        Err(CapitalError::UnsupportedOperation(format!(
            "{} does not read unauthenticated payment notifications",
            self.name()
        )))
    }

    /// Confirms a one-off payment by querying the provider.
    ///
    /// `request` is the server-side request the checkout was created from,
    /// loaded by the application from its own storage with
    /// `notification.reference()`. A paid result requires the provider to
    /// confirm the payment and to report the request's total amount and
    /// currency ([`OneOffPayment::paid`]). The default returns
    /// [`CapitalError::UnsupportedOperation`]. Since 13.0.0-alpha.5.
    async fn confirm_one_off_payment(
        &self,
        _request: &OneOffCheckoutRequest,
        _notification: &PaymentNotification,
    ) -> Result<OneOffPayment, CapitalError> {
        Err(CapitalError::UnsupportedOperation(format!(
            "{} does not confirm one-off payments at the source",
            self.name()
        )))
    }

    /// Verify the signature and extract subscription data from webhook request.
    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError>;

    /// Create a customer portal session URL.
    async fn create_customer_portal(
        &self,
        customer_email: &str,
        return_url: &str,
    ) -> Result<String, CapitalError>;

    /// Cancel a subscription immediately.
    async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), CapitalError>;

    /// Pause a subscription.
    async fn pause_subscription(&self, subscription_id: &str) -> Result<(), CapitalError>;

    /// Report metered usage for a subscription.
    async fn report_usage(
        &self,
        subscription_id: &str,
        metric: &str,
        quantity: u64,
    ) -> Result<(), CapitalError>;

    /// Apply a coupon to an active subscription.
    async fn apply_coupon(
        &self,
        subscription_id: &str,
        coupon_code: &str,
    ) -> Result<(), CapitalError>;

    /// Extend a trial for a subscription by setting a new end timestamp.
    async fn extend_trial(
        &self,
        subscription_id: &str,
        trial_ends_at: i64,
    ) -> Result<(), CapitalError>;
}

/// Helper to url-encode string values without external dependencies.
pub fn url_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                let _ = std::fmt::Write::write_fmt(&mut encoded, format_args!("%{:02X}", b));
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests;
