use super::{PaddleProvider, execute_http, http_client, read_http_json, validate_checkout_url};
use crate::paddle_checkout::{id, invalid};
use crate::{
    CapitalError, PaddleCheckoutRequest, PaddleCheckoutSession, PaddleCustomerReceipt,
    PaddleCustomerRequest,
};
use reqwest::Method;
use serde_json::{Value, json};

impl PaddleProvider {
    /// Creates a customer with immutable owner/attempt metadata. Never looks up
    /// or claims an existing customer by email. Persist intent before dispatch;
    /// an uncertain result requires read-only/operator reconciliation, not a
    /// blind retry: Paddle does not provide arbitrary client idempotency keys.
    pub async fn create_customer(
        &self,
        request: &PaddleCustomerRequest,
    ) -> Result<PaddleCustomerReceipt, CapitalError> {
        if self.offline() {
            return Ok(PaddleCustomerReceipt {
                id: mock_id("ctm_", request.request_digest()),
                digest: request.request_digest(),
                sandbox: None,
            });
        }
        let response = self.billing_json(Method::POST, "/customers", Some(json!({
            "email": request.email, "custom_data": {"rullst_owner_reference": request.owner, "rullst_attempt_reference": request.attempt}
        })), "create customer").await?;
        parse_customer(request, &response["data"], None, true, self.sandbox)
    }

    /// Recovers an independently located customer ID after a lost creation
    /// response. Original owner and provisioning attempt must match; current
    /// contact email may have changed. No mutation or email search is performed.
    pub async fn retrieve_bound_customer(
        &self,
        request: &PaddleCustomerRequest,
        customer_id: &str,
    ) -> Result<PaddleCustomerReceipt, CapitalError> {
        if !id(customer_id, "ctm_") {
            return Err(invalid());
        }
        if self.offline() {
            let receipt = self.create_customer(request).await?;
            return if receipt.id() == customer_id {
                Ok(receipt)
            } else {
                Err(mismatch())
            };
        }
        let response = self
            .billing_json(
                Method::GET,
                &format!("/customers/{customer_id}"),
                None,
                "retrieve customer",
            )
            .await?;
        parse_customer(
            request,
            &response["data"],
            Some(customer_id),
            false,
            self.sandbox,
        )
    }

    /// Creates one automatically-collected subscription transaction. Customer
    /// ownership is checked before the mutation. The returned URL launches the
    /// host's configured Paddle.js page, whose approval/setup remain external.
    pub async fn create_transaction_checkout(
        &self,
        request: &PaddleCheckoutRequest,
    ) -> Result<PaddleCheckoutSession, CapitalError> {
        if self.offline() {
            return Ok(mock_checkout(request));
        }
        let customer = self
            .billing_json(
                Method::GET,
                &format!("/customers/{}", request.customer),
                None,
                "verify checkout customer",
            )
            .await?;
        let customer = &customer["data"];
        if customer["id"].as_str() != Some(&request.customer)
            || customer["status"].as_str() != Some("active")
            || customer["custom_data"]["rullst_owner_reference"].as_str() != Some(&request.owner)
        {
            return Err(mismatch());
        }
        let response = self
            .billing_json(
                Method::POST,
                "/transactions",
                Some(checkout_body(request)),
                "create transaction checkout",
            )
            .await?;
        parse_transaction(request, &response["data"], None, self.sandbox, true)
    }

    /// Retrieves a known transaction after a lost response or signed event.
    /// Account/environment and transaction ID must come from durable host state.
    pub async fn retrieve_transaction_checkout(
        &self,
        request: &PaddleCheckoutRequest,
        transaction_id: &str,
    ) -> Result<PaddleCheckoutSession, CapitalError> {
        if !id(transaction_id, "txn_") {
            return Err(invalid());
        }
        if self.offline() {
            let receipt = mock_checkout(request);
            return if receipt.id() == transaction_id {
                Ok(receipt)
            } else {
                Err(mismatch())
            };
        }
        let response = self
            .billing_json(
                Method::GET,
                &format!("/transactions/{transaction_id}"),
                None,
                "retrieve transaction checkout",
            )
            .await?;
        parse_transaction(
            request,
            &response["data"],
            Some(transaction_id),
            self.sandbox,
            false,
        )
    }

    pub(super) fn offline(&self) -> bool {
        self.api_key.is_empty() || self.api_key.starts_with("mock_")
    }

    pub(super) async fn billing_json(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        operation: &'static str,
    ) -> Result<Value, CapitalError> {
        let response = execute_http(
            self.billing_request(method, path, body, operation)?,
            "paddle",
            operation,
        )
        .await?;
        read_http_json(response, "paddle", operation).await
    }

    fn billing_request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        operation: &'static str,
    ) -> Result<reqwest::Request, CapitalError> {
        let base = if self.sandbox {
            "https://sandbox-api.paddle.com"
        } else {
            "https://api.paddle.com"
        };
        #[cfg(test)]
        let base = self.fixture_url.as_deref().unwrap_or(base);
        let mut request = http_client()?
            .request(method, format!("{base}{path}"))
            .bearer_auth(&self.api_key)
            .header("Paddle-Version", "1");
        if let Some(body) = body {
            request = request.json(&body);
        }
        request
            .build()
            .map_err(|_| crate::ProviderFailure::request_build("paddle", operation).into())
    }
}

fn checkout_body(request: &PaddleCheckoutRequest) -> Value {
    json!({"items": [{"price_id": request.price, "quantity": 1}], "customer_id": request.customer,
        "collection_mode": "automatic", "custom_data": {"rullst_owner_reference": request.owner, "rullst_attempt_reference": request.attempt},
        "checkout": {"url": request.payment_link}})
}
fn parse_customer(
    request: &PaddleCustomerRequest,
    data: &Value,
    expected_id: Option<&str>,
    created: bool,
    sandbox: bool,
) -> Result<PaddleCustomerReceipt, CapitalError> {
    let customer_id = data["id"]
        .as_str()
        .filter(|value| id(value, "ctm_"))
        .ok_or_else(mismatch)?;
    if expected_id.is_some_and(|expected| customer_id != expected)
        || data["status"].as_str() != Some("active")
        || data["custom_data"]["rullst_owner_reference"].as_str() != Some(&request.owner)
        || data["custom_data"]["rullst_attempt_reference"].as_str() != Some(&request.attempt)
        || (created && data["email"].as_str() != Some(&request.email))
    {
        return Err(mismatch());
    }
    Ok(PaddleCustomerReceipt {
        id: customer_id.into(),
        digest: request.request_digest(),
        sandbox: Some(sandbox),
    })
}
fn parse_transaction(
    request: &PaddleCheckoutRequest,
    data: &Value,
    expected_id: Option<&str>,
    sandbox: bool,
    created: bool,
) -> Result<PaddleCheckoutSession, CapitalError> {
    let transaction = data["id"]
        .as_str()
        .filter(|value| id(value, "txn_"))
        .ok_or_else(mismatch)?;
    let status = data["status"].as_str().ok_or_else(mismatch)?;
    let open = matches!(status, "draft" | "ready" | "billed");
    if (created && !matches!(status, "draft" | "ready"))
        || (!open && !matches!(status, "paid" | "completed" | "canceled" | "past_due"))
        || expected_id.is_some_and(|expected| transaction != expected)
        || data["origin"].as_str() != Some("api")
        || data["collection_mode"].as_str() != Some("automatic")
        || data["customer_id"].as_str() != Some(&request.customer)
        || data["custom_data"]["rullst_owner_reference"].as_str() != Some(&request.owner)
        || data["custom_data"]["rullst_attempt_reference"].as_str() != Some(&request.attempt)
    {
        return Err(mismatch());
    }
    price_matches(request, &data["items"])?;
    let url = if data["checkout"]["url"].is_null() {
        None
    } else {
        Some(bound_payment_url(
            request,
            transaction,
            data["checkout"]["url"].as_str().ok_or_else(mismatch)?,
        )?)
    };
    if open && url.is_none() {
        return Err(mismatch());
    }
    let subscription = match &data["subscription_id"] {
        Value::Null => None,
        Value::String(value) if id(value, "sub_") => Some(value.clone()),
        _ => return Err(mismatch()),
    };
    if created && subscription.is_some() {
        return Err(mismatch());
    }
    Ok(PaddleCheckoutSession {
        id: transaction.into(),
        url,
        status: status.into(),
        subscription,
        digest: request.request_digest(),
        sandbox: Some(sandbox),
    })
}
pub(super) fn price_matches(
    request: &PaddleCheckoutRequest,
    items: &Value,
) -> Result<(), CapitalError> {
    let items = items
        .as_array()
        .filter(|items| items.len() == 1)
        .ok_or_else(mismatch)?;
    let cycle = &items[0]["price"]["billing_cycle"];
    if items[0]["price"]["id"].as_str() != Some(&request.price)
        || items[0]["quantity"].as_u64() != Some(1)
        || !matches!(
            cycle["interval"].as_str(),
            Some("day" | "week" | "month" | "year")
        )
        || !cycle["frequency"]
            .as_u64()
            .is_some_and(|frequency| frequency > 0)
    {
        return Err(mismatch());
    }
    Ok(())
}
fn bound_payment_url(
    request: &PaddleCheckoutRequest,
    transaction: &str,
    text: &str,
) -> Result<String, CapitalError> {
    validate_checkout_url("paddle", text)?;
    let mut actual = reqwest::Url::parse(text).map_err(|_| mismatch())?;
    let pairs = actual.query_pairs().collect::<Vec<_>>();
    if pairs.len() != 1 || pairs[0].0 != "_ptxn" || pairs[0].1 != transaction {
        return Err(mismatch());
    }
    actual.set_query(None);
    if actual.as_str() != request.payment_link {
        return Err(mismatch());
    }
    Ok(text.into())
}
fn mock_id(prefix: &str, digest: [u8; 32]) -> String {
    format!("{prefix}{}", &hex::encode(digest)[..26])
}
fn mock_checkout(request: &PaddleCheckoutRequest) -> PaddleCheckoutSession {
    let digest = request.request_digest();
    let id = mock_id("txn_", digest);
    PaddleCheckoutSession {
        url: Some(format!("https://mock.paddle.invalid/{id}")),
        id,
        status: "mock".into(),
        subscription: None,
        digest,
        sandbox: None,
    }
}
pub(super) fn mismatch() -> CapitalError {
    crate::ProviderFailure::contract_mismatch("paddle", "bound billing contract").into()
}

#[cfg(test)]
#[path = "paddle_checkout_tests.rs"]
mod tests;
