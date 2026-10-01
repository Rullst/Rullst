//! Read-only recovery for persisted provisioning and checkout attempts.
use super::{execute_http, http_client, read_http_json, stripe_contract, url_encode};
use crate::{CapitalError, StripeCheckoutRequest, StripeCustomerReceipt, StripeCustomerRequest};
use serde_json::Value;

/// Latest provider checkout state, with ownership, attempt and price bindings checked.
#[derive(Clone)]
pub struct StripeCheckoutSnapshot {
    id: String,
    status: String,
    url: Option<String>,
    subscription: Option<String>,
    expires_at: i64,
}
impl StripeCheckoutSnapshot {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn status(&self) -> &str {
        &self.status
    }
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
    pub fn subscription_id(&self) -> Option<&str> {
        self.subscription.as_deref()
    }
    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }
}
impl std::fmt::Debug for StripeCheckoutSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripeCheckoutSnapshot")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

impl super::StripeProvider {
    /// Creates a portal for an already persisted customer ID. No email lookup
    /// occurs; the caller must authorize the customer/account binding first.
    pub async fn create_bound_customer_portal(
        &self,
        customer: &str,
        return_url: &str,
    ) -> Result<String, CapitalError> {
        if !stripe_contract::valid_reference(customer, "cus_", 200) {
            return Err(invalid_input(
                "Stripe customer ID must be a bounded cus_ reference",
            ));
        }
        super::validate_checkout_url("portal-return", return_url).map_err(|_| {
            CapitalError::ConfigurationError(
                "Stripe portal return URL must be a bounded credential-free HTTPS URL without a fragment".into(),
            )
        })?;
        if self.usage_api_key().is_empty() || self.usage_api_key().starts_with("mock_") {
            return Ok(format!("https://mock.stripe.invalid/portal/{customer}"));
        }
        let request = http_client()?
            .post("https://api.stripe.com/v1/billing_portal/sessions")
            .bearer_auth(self.usage_api_key())
            .header("Stripe-Version", stripe_contract::API_VERSION)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!(
                "customer={}&return_url={}",
                url_encode(customer),
                url_encode(return_url)
            ))
            .build()
            .map_err(|_| mismatch())?;
        let response = execute_http(request, "stripe", "create bound portal").await?;
        let body: Value = read_http_json(response, "stripe", "create bound portal").await?;
        if body["object"].as_str() != Some("billing_portal.session")
            || body["customer"].as_str() != Some(customer)
            || body["return_url"].as_str() != Some(return_url)
            || body["livemode"].as_bool() != stripe_contract::credential_mode(self.usage_api_key())
        {
            return Err(mismatch());
        }
        super::validate_checkout_url("stripe", body["url"].as_str().ok_or_else(mismatch)?)
    }

    /// Checks the platform account before using persisted billing state.
    /// Connected-account impersonation is outside this contract.
    pub async fn verify_account(&self, account: &str, livemode: bool) -> Result<(), CapitalError> {
        if !stripe_contract::valid_reference(account, "acct_", 200) {
            return Err(invalid_input(
                "Stripe account ID must be a bounded acct_ reference",
            ));
        }
        self.require_read_mode(livemode)?;
        let body = self.billing_read("account").await?;
        if body["id"].as_str() != Some(account) || body["object"].as_str() != Some("account") {
            return Err(mismatch());
        }
        Ok(())
    }

    /// Recovers a customer by an operator-supplied ID, checking original metadata
    /// and credential mode. Mutable email never establishes ownership.
    pub async fn retrieve_bound_customer(
        &self,
        request: &StripeCustomerRequest,
        customer: &str,
    ) -> Result<StripeCustomerReceipt, CapitalError> {
        if !stripe_contract::valid_reference(customer, "cus_", 200) {
            return Err(invalid_input(
                "Stripe customer ID must be a bounded cus_ reference",
            ));
        }
        let body = self.billing_read(&format!("customers/{customer}")).await?;
        let result = super::stripe_customer::parse_response(
            request,
            &body,
            stripe_contract::credential_mode(self.usage_api_key()),
        )?;
        if result.id() != customer {
            return Err(mismatch());
        }
        Ok(result)
    }

    /// Recovers a lost response by exact opaque metadata. Search is eventually
    /// consistent: absence never authorizes recreation after key retention.
    pub async fn find_bound_customer(
        &self,
        request: &StripeCustomerRequest,
    ) -> Result<Option<StripeCustomerReceipt>, CapitalError> {
        let query = format!(
            "metadata['rullst_owner_reference']:'{}'",
            request.owner_reference()
        );
        let body = self
            .billing_read(&format!(
                "customers/search?limit=2&query={}",
                url_encode(&query)
            ))
            .await?;
        let items = bounded_list(&body)?;
        if items.len() > 1 {
            return Err(mismatch());
        }
        items
            .first()
            .map(|body| {
                super::stripe_customer::parse_response(
                    request,
                    body,
                    stripe_contract::credential_mode(self.usage_api_key()),
                )
            })
            .transpose()
    }

    pub async fn retrieve_checkout(
        &self,
        request: &StripeCheckoutRequest,
        session: &str,
        livemode: bool,
    ) -> Result<StripeCheckoutSnapshot, CapitalError> {
        if !stripe_contract::valid_reference(session, "cs_", 255) {
            return Err(invalid_input(
                "Stripe checkout session ID must be a bounded cs_ reference",
            ));
        }
        self.require_read_mode(livemode)?;
        let body = self
            .billing_read(&format!("checkout/sessions/{session}?expand[0]=line_items"))
            .await?;
        let snapshot = parse_checkout(request, &body, livemode)?;
        if snapshot.id() != session {
            return Err(mismatch());
        }
        Ok(snapshot)
    }

    /// Bounded recovery of an uncertain checkout or an early subscription event.
    /// The list must be exhaustive. Absence never permits retrying an old mutation.
    pub async fn find_checkout(
        &self,
        request: &StripeCheckoutRequest,
        created_after: i64,
        subscription: Option<&str>,
        livemode: bool,
    ) -> Result<Option<StripeCheckoutSnapshot>, CapitalError> {
        if created_after <= 0
            || subscription.is_some_and(|id| !stripe_contract::valid_reference(id, "sub_", 200))
        {
            return Err(invalid_input(
                "checkout recovery requires a positive creation time and a bounded sub_ reference",
            ));
        }
        self.require_read_mode(livemode)?;
        let mut path = format!(
            "checkout/sessions?customer={}&created[gte]={}&limit=100&expand[0]=data.line_items",
            url_encode(request.customer_id()),
            created_after.saturating_sub(300)
        );
        if let Some(subscription) = subscription {
            path.push_str(&format!("&subscription={subscription}"));
        }
        let body = self.billing_read(&path).await?;
        let items = bounded_list(&body)?;
        let mut found = None;
        for item in items {
            if item["metadata"]["rullst_attempt_reference"].as_str()
                == Some(request.idempotency_key())
            {
                let snapshot = parse_checkout(request, item, livemode)?;
                if found.is_some()
                    || subscription.is_some_and(|id| snapshot.subscription_id() != Some(id))
                {
                    return Err(mismatch());
                }
                found = Some(snapshot);
            }
        }
        Ok(found)
    }

    // Local input, offline and mode problems are classified before any request;
    // only provider responses produce a contract mismatch.
    fn read_mode(&self) -> Result<bool, CapitalError> {
        let key = self.usage_api_key();
        if key.is_empty() || key.starts_with("mock_") {
            return Err(CapitalError::UnsupportedOperation(
                "Stripe reconciliation reads have no offline fixture; use an sk_test_ or rk_test_ key"
                    .into(),
            ));
        }
        stripe_contract::credential_mode(key).ok_or_else(|| {
            invalid_input("Stripe reconciliation reads require an sk_ or rk_ live or test key")
        })
    }

    fn require_read_mode(&self, livemode: bool) -> Result<(), CapitalError> {
        if self.read_mode()? != livemode {
            return Err(invalid_input(
                "requested Stripe live/test mode does not match the configured key",
            ));
        }
        Ok(())
    }

    async fn billing_read(&self, path: &str) -> Result<Value, CapitalError> {
        self.read_mode()?;
        let request = http_client()?
            .get(format!("https://api.stripe.com/v1/{path}"))
            .bearer_auth(self.usage_api_key())
            .header("Stripe-Version", stripe_contract::API_VERSION)
            .build()
            .map_err(|_| mismatch())?;
        read_http_json(
            execute_http(request, "stripe", "reconcile billing").await?,
            "stripe",
            "reconcile billing",
        )
        .await
    }
}

fn bounded_list(body: &Value) -> Result<&Vec<Value>, CapitalError> {
    if !matches!(body["object"].as_str(), Some("list" | "search_result"))
        || body["has_more"].as_bool() != Some(false)
    {
        return Err(mismatch());
    }
    body["data"]
        .as_array()
        .filter(|items| items.len() <= 100)
        .ok_or_else(mismatch)
}

fn parse_checkout(
    request: &StripeCheckoutRequest,
    body: &Value,
    livemode: bool,
) -> Result<StripeCheckoutSnapshot, CapitalError> {
    let id = body["id"]
        .as_str()
        .filter(|id| stripe_contract::valid_reference(id, "cs_", 255))
        .ok_or_else(mismatch)?;
    let status = body["status"]
        .as_str()
        .filter(|status| matches!(*status, "open" | "complete" | "expired"))
        .ok_or_else(mismatch)?;
    let items = bounded_list(&body["line_items"])?;
    if body["object"].as_str() != Some("checkout.session")
        || body["mode"].as_str() != Some("subscription")
        || body["customer"].as_str() != Some(request.customer_id())
        || body["client_reference_id"].as_str() != Some(request.owner_reference())
        || body["metadata"]["rullst_owner_reference"].as_str() != Some(request.owner_reference())
        || body["metadata"]["rullst_attempt_reference"].as_str() != Some(request.idempotency_key())
        || body["livemode"].as_bool() != Some(livemode)
        || body["success_url"].as_str() != Some(request.success_url())
        || body["cancel_url"].as_str() != Some(request.cancel_url())
        || items.len() != 1
        || items[0]["quantity"].as_i64() != Some(1)
        || items[0]["price"]["id"].as_str() != Some(request.price_id())
        || items[0]["price"]["type"].as_str() != Some("recurring")
    {
        return Err(mismatch());
    }
    let subscription = match body["subscription"].as_str() {
        Some(id) if stripe_contract::valid_reference(id, "sub_", 200) => Some(id.to_owned()),
        None if body["subscription"].is_null() && status != "complete" => None,
        _ => return Err(mismatch()),
    };
    let url = if status == "open" {
        Some(super::validate_checkout_url(
            "stripe",
            body["url"].as_str().ok_or_else(mismatch)?,
        )?)
    } else {
        None
    };
    Ok(StripeCheckoutSnapshot {
        id: id.into(),
        status: status.into(),
        url,
        subscription,
        expires_at: body["expires_at"]
            .as_i64()
            .filter(|time| *time > 0)
            .ok_or_else(mismatch)?,
    })
}
fn mismatch() -> CapitalError {
    crate::ProviderFailure::contract_mismatch("stripe", "reconcile billing").into()
}

fn invalid_input(message: &str) -> CapitalError {
    CapitalError::ConfigurationError(message.into())
}

#[cfg(test)]
#[path = "stripe_reconcile_tests.rs"]
mod tests;
