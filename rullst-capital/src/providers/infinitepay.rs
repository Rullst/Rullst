use super::{
    BillingProvider, SubscriptionStatus, WebhookEvent, WebhookVerificationMode,
    verify_explicit_mock_signature, webhook_mode_from_secret,
};
use crate::error::CapitalError;
use crate::{OneOffCheckout, OneOffCheckoutRequest, OneOffPayment, PaymentNotification};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

mod api;

/// InfinitePay's public Checkout API.
const API_BASE: &str = "https://api.checkout.infinitepay.io";

/// Experimental adapter for InfinitePay's public
/// [Checkout API](https://www.infinitepay.io/checkout-documentacao) (CloudWalk,
/// Brazil). Since 13.0.0-alpha.5 it performs real operations.
///
/// Build it with [`Self::checkout`] from the merchant's handle (InfiniteTag):
///
/// - [`BillingProvider::create_one_off_checkout`] creates a payment link with
///   `POST https://api.checkout.infinitepay.io/links`: one item priced in
///   cents from the server-side [`OneOffCheckoutRequest`] (BRL, at least
///   R$ 1.00 per unit), the request's reference as `order_nsu`, its success
///   URL as `redirect_url` and the configured `webhook_url`. With an
///   `inchk_…` API key ([`Self::with_api_key`]) the link also accepts a single
///   payment. InfinitePay has no cancel URL. Until the merchant enables
///   external checkout ("Checkout externo") for the InfiniteTag in the
///   InfinitePay app, and for a tag that does not exist, InfinitePay answers
///   404 and the [`ProviderFailure`](crate::ProviderFailure)'s
///   [`reason`](crate::ProviderFailure::reason) is
///   `external_checkout_not_enabled`.
/// - The webhook is **unsigned**, so the adapter's mode is
///   [`WebhookVerificationMode::ConfirmedAtSource`]:
///   [`BillingProvider::parse_payment_notification`] reads only `order_nsu`,
///   `transaction_nsu` and `invoice_slug`, and
///   [`BillingProvider::confirm_one_off_payment`] calls
///   `POST /payment_check` and reports the payment paid only when InfinitePay
///   confirms it and its `amount` equals the request's total (`paid_amount`
///   may be higher when the buyer pays instalment fees).
/// - InfinitePay has no subscriptions, customer portal, metered usage,
///   coupons, trials or sandbox: those operations, and plan-only
///   `create_checkout_session` with a live handle, return
///   [`CapitalError::UnsupportedOperation`].
///
/// An empty or `mock_*` handle selects deterministic offline fixtures: a
/// `mock.infinitepay.invalid` checkout URL and an
/// [`OneOffPaymentState::Mock`](crate::OneOffPaymentState::Mock) confirmation
/// that never proves payment. [`Self::new`] keeps the 13.0.0-alpha.4
/// constructor, which has no handle: it runs the offline fixtures with
/// `mock_*` credentials and refuses live operations.
///
/// Fees, settlement schedules, instalment terms and merchant eligibility belong
/// to the merchant's InfinitePay account. The adapter stays experimental until
/// it is validated against a live account.
pub struct InfinitePayProvider {
    /// `None` for the legacy [`Self::new`] constructor.
    handle: Option<String>,
    /// The legacy API key, or the optional `inchk_` key ("" when unset).
    api_key: String,
    /// The legacy webhook secret; unused by [`Self::checkout`].
    webhook_secret: String,
    webhook_url: Option<String>,
    api_base: String,
}

impl InfinitePayProvider {
    /// The 13.0.0-alpha.4 constructor, kept for source compatibility.
    ///
    /// It has no merchant handle. With an empty or `mock_*` API key and a
    /// `mock_*` webhook secret it runs the offline fixtures (a mock checkout,
    /// a mock-signed webhook); live operations fail. Use [`Self::checkout`].
    pub fn new(api_key: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            handle: None,
            api_key: api_key.into(),
            webhook_secret: webhook_secret.into(),
            webhook_url: None,
            api_base: API_BASE.to_string(),
        }
    }

    /// An adapter for the merchant's handle (InfiniteTag, with or without the
    /// leading `$`). An empty or `mock_*` handle selects the offline fixtures.
    /// Since 13.0.0-alpha.5.
    pub fn checkout(handle: impl Into<String>) -> Result<Self, CapitalError> {
        let handle = handle.into();
        let handle = handle.trim().trim_start_matches('$').to_string();
        let valid = handle.len() <= 64
            && handle
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
        if !valid {
            return Err(CapitalError::ConfigurationError(
                "InfinitePay handle must be the InfiniteTag: up to 64 ASCII letters, digits, '-', '_' or '.', without spaces".into(),
            ));
        }
        Ok(Self {
            handle: Some(handle),
            api_key: String::new(),
            webhook_secret: String::new(),
            webhook_url: None,
            api_base: API_BASE.to_string(),
        })
    }

    /// Sends the `inchk_…` API key in `X-Api-Key`, which lets each link accept
    /// a single payment (`options.single_payment`). Keep it on the server.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Result<Self, CapitalError> {
        let api_key = api_key.into();
        if !api_key.starts_with("inchk_")
            || api_key.len() > 200
            || !api_key.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(CapitalError::ConfigurationError(
                "InfinitePay API key must start with inchk_ and contain at most 200 visible ASCII characters".into(),
            ));
        }
        self.api_key = api_key;
        Ok(self)
    }

    /// The URL InfinitePay notifies after a payment (`webhook_url` of each
    /// link). It must be HTTPS when a live handle creates a link; mount it
    /// behind Capital's webhook middleware or call
    /// [`BillingProvider::parse_payment_notification`] in the handler.
    pub fn with_webhook_url(mut self, url: impl Into<String>) -> Result<Self, CapitalError> {
        let url = url.into();
        super::validate_redirect_url(&url).map_err(|_| {
            CapitalError::ConfigurationError(
                "InfinitePay webhook URL must be a bounded credential-free HTTPS URL without a fragment".into(),
            )
        })?;
        self.webhook_url = Some(url);
        Ok(self)
    }

    /// Whether the adapter runs the deterministic offline fixtures.
    pub fn is_mock(&self) -> bool {
        match &self.handle {
            Some(handle) => handle.is_empty() || handle.starts_with("mock_"),
            None => self.api_key.is_empty() || self.api_key.starts_with("mock_"),
        }
    }

    /// Verifies the legacy mock signature of [`Self::new`]'s offline fixture.
    ///
    /// InfinitePay does not sign its webhook, so a live secret is refused;
    /// confirm payments with [`BillingProvider::confirm_one_off_payment`].
    pub fn verify_signature(
        &self,
        _payload: &[u8],
        signature_hex: &str,
    ) -> Result<(), CapitalError> {
        if self.webhook_verification_mode()? == WebhookVerificationMode::Mock {
            return verify_explicit_mock_signature(
                self.name(),
                &self.webhook_secret,
                signature_hex,
            );
        }
        Err(unsigned_notification())
    }

    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }

    fn live_handle(&self) -> Result<&str, CapitalError> {
        self.handle.as_deref().ok_or_else(|| {
            CapitalError::ConfigurationError(
                "InfinitePayProvider::new has no merchant handle; build the adapter with InfinitePayProvider::checkout(handle)".into(),
            )
        })
    }
}

#[async_trait]
impl BillingProvider for InfinitePayProvider {
    fn name(&self) -> &'static str {
        "infinitepay"
    }

    /// [`WebhookVerificationMode::ConfirmedAtSource`], except for the legacy
    /// [`Self::new`] fixture with a `mock_*` secret, which is `Mock`.
    fn webhook_verification_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        if self.handle.is_some() {
            return Ok(WebhookVerificationMode::ConfirmedAtSource);
        }
        match webhook_mode_from_secret(self.name(), &self.webhook_secret)? {
            WebhookVerificationMode::Mock => Ok(WebhookVerificationMode::Mock),
            _ => Ok(WebhookVerificationMode::ConfirmedAtSource),
        }
    }

    async fn create_checkout_session(
        &self,
        customer_email: &str,
        plan_id: &str,
        _redirect_url: &str,
    ) -> Result<String, CapitalError> {
        if customer_email.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Customer email cannot be empty".to_string(),
            ));
        }
        if plan_id.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Plan ID cannot be empty".to_string(),
            ));
        }
        if self.is_mock() {
            return Ok(super::fixture::checkout_url(self.name(), plan_id));
        }
        Err(CapitalError::UnsupportedOperation(
            "InfinitePay has no plans; create a link priced on the server with create_one_off_checkout".into(),
        ))
    }

    async fn create_one_off_checkout(
        &self,
        request: &OneOffCheckoutRequest,
    ) -> Result<OneOffCheckout, CapitalError> {
        api::validate_request(request)?;
        if self.is_mock() {
            return Ok(OneOffCheckout::mock(self.name(), request));
        }
        let handle = self.live_handle()?;
        api::create_link(self, handle, request).await
    }

    fn parse_payment_notification(
        &self,
        payload: &[u8],
        _headers: &HashMap<String, String>,
    ) -> Result<PaymentNotification, CapitalError> {
        api::parse_notification(payload)
    }

    async fn confirm_one_off_payment(
        &self,
        request: &OneOffCheckoutRequest,
        notification: &PaymentNotification,
    ) -> Result<OneOffPayment, CapitalError> {
        notification.require_reference_of(request)?;
        if self.is_mock() {
            return Ok(OneOffPayment::mock(self.name(), request));
        }
        let handle = self.live_handle()?;
        api::payment_check(self, handle, request, notification).await
    }

    /// Only the legacy [`Self::new`] fixture with a `mock_*` secret reads a
    /// mock-signed body. InfinitePay's real webhook is unsigned, so any other
    /// configuration returns [`CapitalError::UnsupportedOperation`]: read it
    /// with [`BillingProvider::parse_payment_notification`] and confirm it.
    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        if self.webhook_verification_mode()? != WebhookVerificationMode::Mock {
            return Err(unsigned_notification());
        }
        let signature = headers
            .get("x-signature")
            .or_else(|| headers.get("x-infinitepay-signature"))
            .ok_or_else(|| {
                CapitalError::InvalidSignature("Missing X-Signature header".to_string())
            })?;
        self.verify_signature(payload, signature)?;
        mock_event(payload)
    }

    async fn create_customer_portal(
        &self,
        _customer_email: &str,
        _return_url: &str,
    ) -> Result<String, CapitalError> {
        Err(unsupported("a customer portal"))
    }

    async fn cancel_subscription(&self, _subscription_id: &str) -> Result<(), CapitalError> {
        Err(unsupported("subscriptions"))
    }

    async fn pause_subscription(&self, _subscription_id: &str) -> Result<(), CapitalError> {
        Err(unsupported("subscriptions"))
    }

    async fn report_usage(
        &self,
        _subscription_id: &str,
        _metric: &str,
        _quantity: u64,
    ) -> Result<(), CapitalError> {
        Err(unsupported("metered usage"))
    }

    async fn apply_coupon(
        &self,
        _subscription_id: &str,
        _coupon_code: &str,
    ) -> Result<(), CapitalError> {
        Err(unsupported("coupons"))
    }

    async fn extend_trial(
        &self,
        _subscription_id: &str,
        _trial_ends_at: i64,
    ) -> Result<(), CapitalError> {
        Err(unsupported("trials"))
    }
}

/// The legacy offline fixture body of [`InfinitePayProvider::new`].
fn mock_event(payload: &[u8]) -> Result<WebhookEvent, CapitalError> {
    let json: Value = serde_json::from_slice(payload).map_err(|error| {
        CapitalError::PayloadParseError(format!("Invalid JSON payload: {error}"))
    })?;
    let text = |paths: &[&[&str]], default: &str| {
        paths
            .iter()
            .find_map(|path| {
                path.iter()
                    .try_fold(&json, |value, key| value.get(*key))
                    .and_then(Value::as_str)
            })
            .unwrap_or(default)
            .to_string()
    };
    let status = json["status"]
        .as_str()
        .or_else(|| json["event"].as_str())
        .filter(|status| !status.trim().is_empty())
        .ok_or_else(|| {
            CapitalError::PayloadParseError("Webhook status is missing or invalid".into())
        })?;
    Ok(WebhookEvent {
        subscription_id: text(&[&["id"], &["transaction_id"], &["data", "id"]], ""),
        customer_id: text(&[&["customer", "id"], &["customer_id"]], ""),
        customer_email: text(&[&["customer", "email"], &["customer_email"]], ""),
        plan_id: text(&[&["plan_id"], &["order_id"]], "default"),
        status: SubscriptionStatus::parse_status(status),
        ends_at: json["ends_at"].as_i64(),
    })
}

fn unsupported(feature: &str) -> CapitalError {
    CapitalError::UnsupportedOperation(format!("InfinitePay's Checkout API has no {feature}"))
}

fn unsigned_notification() -> CapitalError {
    CapitalError::UnsupportedOperation(
        "InfinitePay webhooks are unsigned; read them with parse_payment_notification and confirm the payment with confirm_one_off_payment".into(),
    )
}

#[cfg(test)]
#[path = "infinitepay_tests.rs"]
mod tests;
