use super::{execute_http, http_client, read_http_json, validate_checkout_url};
use crate::{CapitalError, PolarCheckoutRequest, PolarCheckoutSession};
use serde_json::{Value, json};

const OPERATION: &str = "create product checkout";

impl super::PolarProvider {
    /// Verifies a subscription event against a persisted checkout's opaque
    /// customer and product bindings. Contact email is never an ownership key.
    /// The caller must still namespace by Polar account/environment and commit
    /// its event receipt and application state atomically.
    pub fn verify_checkout_subscription(
        &self,
        request: &PolarCheckoutRequest,
        payload: &[u8],
        headers: &std::collections::HashMap<String, String>,
    ) -> Result<crate::WebhookEvent, CapitalError> {
        use crate::BillingProvider;
        let event = self.handle_webhook(payload, headers)?;
        let json: Value = serde_json::from_slice(payload).map_err(|_| mismatch())?;
        if event.plan_id != request.product_id()
            || json["data"]["customer"]["external_id"].as_str()
                != Some(request.external_customer_id())
        {
            return Err(mismatch());
        }
        Ok(event)
    }

    /// Creates checkout through POST /v1/checkouts/ using current product IDs.
    /// The external customer reference is explicit; the legacy email/price
    /// trait method cannot infer it. No automatic retry is performed.
    pub async fn create_product_checkout(
        &self,
        request: &PolarCheckoutRequest,
    ) -> Result<PolarCheckoutSession, CapitalError> {
        if self.api_key.is_empty() || self.api_key.starts_with("mock_") {
            let encoded = serde_json::to_vec(&body(request)).map_err(|_| mismatch())?;
            let id = hex::encode(ring::digest::digest(&ring::digest::SHA256, &encoded));
            return Ok(PolarCheckoutSession {
                url: format!("https://mock.polar.invalid/{id}"),
                id,
                expires_at: None,
                mock: true,
            });
        }
        let response = execute_http(
            build_request(http_client()?, &self.api_key, self.sandbox, request)?,
            "polar",
            OPERATION,
        )
        .await?;
        parse_response(
            request,
            &read_http_json::<Value>(response, "polar", OPERATION).await?,
        )
    }
}

fn body(request: &PolarCheckoutRequest) -> Value {
    let mut body = json!({"products": [&request.product], "external_customer_id": request.owner,
        "success_url": request.success, "metadata": {"rullst_owner_reference": request.owner}});
    if let Some(email) = &request.email {
        body["customer_email"] = json!(email);
    }
    if let Some(ip) = request.ip {
        body["customer_ip_address"] = json!(ip.to_string());
    }
    body
}

fn build_request(
    client: &reqwest::Client,
    key: &str,
    sandbox: bool,
    request: &PolarCheckoutRequest,
) -> Result<reqwest::Request, CapitalError> {
    let url = if sandbox {
        "https://sandbox-api.polar.sh/v1/checkouts/"
    } else {
        "https://api.polar.sh/v1/checkouts/"
    };
    client
        .post(url)
        .bearer_auth(key)
        .json(&body(request))
        .build()
        .map_err(|_| crate::ProviderFailure::request_build("polar", OPERATION).into())
}

fn parse_response(
    request: &PolarCheckoutRequest,
    response: &Value,
) -> Result<PolarCheckoutSession, CapitalError> {
    let id = response["id"]
        .as_str()
        .filter(|id| crate::polar_checkout::uuid(id))
        .ok_or_else(mismatch)?;
    let products = response["products"].as_array().ok_or_else(mismatch)?;
    if response["status"].as_str() != Some("open")
        || response["external_customer_id"].as_str() != Some(&request.owner)
        || response["success_url"].as_str()
            != Some(request.success.replace("{CHECKOUT_ID}", id).as_str())
        || response["metadata"]["rullst_owner_reference"].as_str() != Some(&request.owner)
        || products.len() != 1
        || products[0]["id"].as_str() != Some(&request.product)
        || (!response["product_id"].is_null()
            && response["product_id"].as_str() != Some(&request.product))
    {
        return Err(mismatch());
    }
    let expires_at = response["expires_at"]
        .as_str()
        .and_then(|time| chrono::DateTime::parse_from_rfc3339(time).ok())
        .map(|time| time.timestamp())
        .filter(|time| *time > 0)
        .ok_or_else(mismatch)?;
    Ok(PolarCheckoutSession {
        id: id.into(),
        url: validate_checkout_url("polar", response["url"].as_str().ok_or_else(mismatch)?)?,
        expires_at: Some(expires_at),
        mock: false,
    })
}
fn mismatch() -> CapitalError {
    crate::ProviderFailure::contract_mismatch("polar", OPERATION).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    const PRODUCT: &str = "1dbfc517-0bbf-4301-9ba8-555ca42b9737";
    fn request() -> PolarCheckoutRequest {
        PolarCheckoutRequest::new(PRODUCT, "owner_opaque", "https://app.example/return").unwrap()
    }
    #[test]
    fn current_wire_contract_and_response_bindings() {
        let input = request().with_trusted_client_ip("203.0.113.5".parse().unwrap());
        for sandbox in [false, true] {
            let wire = build_request(http_client().unwrap(), "secret", sandbox, &input).unwrap();
            assert_eq!(wire.url().path(), "/v1/checkouts/");
            assert_eq!(
                wire.url().host_str(),
                Some(if sandbox {
                    "sandbox-api.polar.sh"
                } else {
                    "api.polar.sh"
                })
            );
            let sent: Value =
                serde_json::from_slice(wire.body().unwrap().as_bytes().unwrap()).unwrap();
            assert_eq!(sent["products"], json!([PRODUCT]));
            assert_eq!(sent["external_customer_id"], "owner_opaque");
            assert_eq!(sent["customer_ip_address"], "203.0.113.5");
            assert!(sent.get("product_price_id").is_none());
        }
        assert!(body(&request()).get("customer_ip_address").is_none());
        let good = json!({"id": PRODUCT, "status": "open", "products": [{"id": PRODUCT}],
            "product_id": PRODUCT, "external_customer_id": "owner_opaque",
            "metadata": {"rullst_owner_reference": "owner_opaque"},
            "success_url": "https://app.example/return", "expires_at": "2027-01-01T00:00:00Z",
            "url": "https://polar.sh/checkout/example"});
        assert!(!parse_response(&input, &good).unwrap().is_mock());
        for field in [
            "external_customer_id",
            "product_id",
            "success_url",
            "status",
            "expires_at",
            "url",
        ] {
            let mut bad = good.clone();
            bad[field] = json!("wrong");
            assert!(parse_response(&input, &bad).is_err(), "{field}");
        }
    }
    #[test]
    fn documented_return_placeholder_is_bound_to_the_returned_checkout_id() {
        let request = PolarCheckoutRequest::new(
            PRODUCT.to_ascii_uppercase(),
            "owner_opaque",
            "https://app.example/return?checkout_id={CHECKOUT_ID}",
        )
        .unwrap();
        assert_eq!(request.product_id(), PRODUCT);
        let mut response = json!({"id": PRODUCT, "status": "open", "products": [{"id": PRODUCT}],
            "external_customer_id": "owner_opaque", "metadata": {"rullst_owner_reference": "owner_opaque"},
            "success_url": format!("https://app.example/return?checkout_id={PRODUCT}"),
            "expires_at": "2027-01-01T00:00:00Z", "url": "https://polar.sh/checkout/example"});
        assert!(parse_response(&request, &response).is_ok());
        response["success_url"] = json!("https://app.example/return?checkout_id=other");
        assert!(parse_response(&request, &response).is_err());
    }

    #[tokio::test]
    async fn deterministic_mock_and_constructor_validation() {
        let provider = super::super::PolarProvider::new("mock_key", "mock_secret");
        let first = provider.create_product_checkout(&request()).await.unwrap();
        assert!(first.is_mock());
        assert_eq!(
            first.id(),
            provider
                .create_product_checkout(&request())
                .await
                .unwrap()
                .id()
        );
        assert!(PolarCheckoutRequest::new("old_price", "owner", "https://app.example").is_err());
        assert!(
            PolarCheckoutRequest::new(PRODUCT, "email@example.com", "https://app.example").is_err()
        );
        assert!(PolarCheckoutRequest::new(PRODUCT, "owner", "http://app.example").is_err());
    }
}
