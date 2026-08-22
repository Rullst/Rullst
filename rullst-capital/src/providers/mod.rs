use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::OnceCell;

pub mod alipay;
pub mod coinbase;
pub mod infinitepay;
pub mod lemonsqueezy;
pub mod mercadopago;
pub mod paddle;
pub mod picpay;
pub mod polar;
pub mod razorpay;
pub mod stripe;
pub mod wise;

pub use alipay::AlipayProvider;
pub use coinbase::{CoinbaseCommerceProvider, CoinbaseProvider};
pub use infinitepay::InfinitePayProvider;
pub use lemonsqueezy::LemonSqueezyProvider;
pub use mercadopago::MercadoPagoProvider;
pub use paddle::PaddleProvider;
pub use picpay::PicPayProvider;
pub use polar::PolarProvider;
pub use razorpay::RazorpayProvider;
pub use stripe::StripeProvider;
pub use wise::WiseProvider;

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

/// Dynamic trait to handle billing provider interactions.
#[async_trait]
pub trait BillingProvider: Send + Sync {
    /// Return the name of the billing provider (e.g. "stripe", "infinitepay", "polar").
    fn name(&self) -> &'static str;

    /// Create a checkout session URL for a customer.
    async fn create_checkout_session(
        &self,
        customer_email: &str,
        plan_id: &str,
        redirect_url: &str,
    ) -> Result<String, CapitalError>;

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

/// The status of an outbound payout/disbursement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayoutStatus {
    Processing,
    OutgoingPaymentSent,
    FundsRefunded,
    Cancelled,
}

/// Unified model representing an outbound payout event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutEvent {
    pub transfer_id: String,
    pub recipient_email: String,
    pub amount_cents: u64,
    pub currency: String,
    pub status: PayoutStatus,
}

/// Dynamic trait for international B2B payouts (e.g. Wise).
#[async_trait]
pub trait PayoutProvider: Send + Sync {
    /// Return the name of the payout provider (e.g. "wise").
    fn name(&self) -> &'static str;

    /// Create a transfer or payout to an international recipient.
    async fn create_transfer(
        &self,
        recipient_email: &str,
        amount_cents: u64,
        currency: &str,
    ) -> Result<String, CapitalError>;

    /// Check transfer status.
    async fn get_transfer_status(&self, transfer_id: &str) -> Result<PayoutStatus, CapitalError>;
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
mod tests {
    use super::*;

    #[test]
    fn test_subscription_status_variants() {
        assert_eq!(SubscriptionStatus::Active.as_str(), "active");
        assert_eq!(SubscriptionStatus::Canceled.as_str(), "canceled");
        assert_eq!(SubscriptionStatus::PastDue.as_str(), "past_due");
        assert_eq!(SubscriptionStatus::Unpaid.as_str(), "unpaid");
        assert_eq!(SubscriptionStatus::Trialing.as_str(), "trialing");
        assert_eq!(SubscriptionStatus::Paused.as_str(), "paused");

        assert_eq!(
            SubscriptionStatus::parse_status("active"),
            SubscriptionStatus::Active
        );
        assert_eq!(
            SubscriptionStatus::parse_status("paid"),
            SubscriptionStatus::Active
        );
        assert_eq!(
            SubscriptionStatus::parse_status("completed"),
            SubscriptionStatus::Active
        );
        assert_eq!(
            SubscriptionStatus::parse_status("approved"),
            SubscriptionStatus::Active
        );
        assert_eq!(
            SubscriptionStatus::parse_status("resolved"),
            SubscriptionStatus::Active
        );
        assert_eq!(
            SubscriptionStatus::parse_status("canceled"),
            SubscriptionStatus::Canceled
        );
        assert_eq!(
            SubscriptionStatus::parse_status("cancelled"),
            SubscriptionStatus::Canceled
        );
        assert_eq!(
            SubscriptionStatus::parse_status("past_due"),
            SubscriptionStatus::PastDue
        );
        assert_eq!(
            SubscriptionStatus::parse_status("unpaid"),
            SubscriptionStatus::Unpaid
        );
        assert_eq!(
            SubscriptionStatus::parse_status("failed"),
            SubscriptionStatus::Unpaid
        );
        assert_eq!(
            SubscriptionStatus::parse_status("rejected"),
            SubscriptionStatus::Unpaid
        );
        assert_eq!(
            SubscriptionStatus::parse_status("expired"),
            SubscriptionStatus::Unpaid
        );
        assert_eq!(
            SubscriptionStatus::parse_status("trialing"),
            SubscriptionStatus::Trialing
        );
        assert_eq!(
            SubscriptionStatus::parse_status("paused"),
            SubscriptionStatus::Paused
        );
        assert_eq!(
            SubscriptionStatus::parse_status("unknown_xyz"),
            SubscriptionStatus::Unpaid
        );
    }

    #[test]
    fn test_url_encoding_characters() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("foo/bar?baz=1"), "foo%2Fbar%3Fbaz%3D1");
        assert_eq!(url_encode("user@domain.com"), "user%40domain.com");
        assert_eq!(url_encode("simple_word-123.test~"), "simple_word-123.test~");
    }

    #[test]
    fn test_global_providers_initialization() {
        let stripe = StripeProvider::new("mock_key", "sec_stripe");
        init_provider(Box::new(stripe));
        assert!(provider().is_some());
        assert_eq!(provider().unwrap().name(), "stripe");

        let wise = WiseProvider::new("mock_token", "sec_wise");
        init_payout_provider(Box::new(wise));
        assert!(payout_provider().is_some());
        assert_eq!(payout_provider().unwrap().name(), "wise");
    }
}
