use super::{http_client, url_encode};
use crate::error::CapitalError;
use crate::{ChargeReceipt, ChargeRequest, ChargeStatus, mock_charge_receipt};
use serde_json::Value;

const PAYMENT_INTENTS_ENDPOINT: &str = "https://api.stripe.com/v1/payment_intents";

pub(super) async fn execute(
    api_key: &str,
    request: &ChargeRequest,
) -> Result<ChargeReceipt, CapitalError> {
    validate_stripe_references(request)?;
    if api_key.is_empty() || api_key.starts_with("mock_") {
        return mock_charge_receipt("stripe", request);
    }

    let outbound = build_request(http_client(), api_key, request)?;
    let response = http_client().execute(outbound).await.map_err(|error| {
        CapitalError::ProviderRequestFailed(format!("Stripe network error: {error}"))
    })?;

    if !response.status().is_success() {
        return Err(CapitalError::ProviderRequestFailed(format!(
            "Stripe direct-charge API error: HTTP {}",
            response.status()
        )));
    }

    let response: Value = response.json().await.map_err(|error| {
        CapitalError::PayloadParseError(format!(
            "failed to parse Stripe direct-charge response: {error}"
        ))
    })?;
    parse_response(request, &response)
}

fn build_request(
    client: &reqwest::Client,
    api_key: &str,
    request: &ChargeRequest,
) -> Result<reqwest::Request, CapitalError> {
    let body = format!(
        "amount={}&currency={}&customer={}&payment_method={}&receipt_email={}&confirm=true&off_session=true&error_on_requires_action=true",
        request.amount_minor(),
        url_encode(request.currency()),
        url_encode(request.customer_id()),
        url_encode(request.payment_method_id()),
        url_encode(request.customer_email()),
    );
    client
        .post(PAYMENT_INTENTS_ENDPOINT)
        .bearer_auth(api_key)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Idempotency-Key", request.idempotency_key())
        .body(body)
        .build()
        .map_err(|error| {
            CapitalError::ProviderRequestFailed(format!(
                "failed to build Stripe direct-charge request: {error}"
            ))
        })
}

fn validate_stripe_references(request: &ChargeRequest) -> Result<(), CapitalError> {
    if !request.customer_id().starts_with("cus_") {
        return Err(CapitalError::InvalidCharge(
            "Stripe customer ID must use the cus_ prefix".to_string(),
        ));
    }
    if !["pm_", "card_", "src_"]
        .iter()
        .any(|prefix| request.payment_method_id().starts_with(prefix))
    {
        return Err(CapitalError::InvalidCharge(
            "Stripe payment method ID must use a reviewed provider-token prefix".to_string(),
        ));
    }
    Ok(())
}

fn parse_response(
    request: &ChargeRequest,
    response: &Value,
) -> Result<ChargeReceipt, CapitalError> {
    let charge_id = response["id"].as_str().ok_or_else(|| {
        CapitalError::PayloadParseError(
            "Stripe direct-charge response omitted its payment-intent ID".to_string(),
        )
    })?;
    let amount_minor = response["amount"].as_u64().ok_or_else(|| {
        CapitalError::PayloadParseError(
            "Stripe direct-charge response omitted its amount".to_string(),
        )
    })?;
    let currency = response["currency"].as_str().ok_or_else(|| {
        CapitalError::PayloadParseError(
            "Stripe direct-charge response omitted its currency".to_string(),
        )
    })?;
    if amount_minor != request.amount_minor() || currency != request.currency() {
        return Err(CapitalError::ProviderRequestFailed(
            "Stripe direct-charge response did not match the requested amount and currency"
                .to_string(),
        ));
    }
    let status = match response["status"].as_str() {
        Some("succeeded") => ChargeStatus::Succeeded,
        Some("processing") => ChargeStatus::Processing,
        Some(status) => {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "Stripe did not accept the direct charge: status {status}"
            )));
        }
        None => {
            return Err(CapitalError::PayloadParseError(
                "Stripe direct-charge response omitted its status".to_string(),
            ));
        }
    };
    if status == ChargeStatus::Succeeded {
        let amount_received = response["amount_received"].as_u64().ok_or_else(|| {
            CapitalError::PayloadParseError(
                "Stripe succeeded response omitted its received amount".to_string(),
            )
        })?;
        if amount_received != request.amount_minor() {
            return Err(CapitalError::ProviderRequestFailed(
                "Stripe succeeded response did not bind the full received amount".to_string(),
            ));
        }
    }

    ChargeReceipt::from_verified_provider_response(
        "stripe",
        charge_id,
        status,
        amount_minor,
        currency,
        request.customer_email(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ChargeRequest {
        ChargeRequest::new(2_500, "BRL", "cus_1", "a@b.co", "pm_1", "order_1").expect("request")
    }

    #[test]
    fn response_is_bound_to_request_and_closed_to_non_accepted_statuses() {
        let request = request();
        let succeeded = serde_json::json!({
            "id": "pi_1",
            "amount": 2_500,
            "amount_received": 2_500,
            "currency": "brl",
            "status": "succeeded"
        });
        assert!(
            parse_response(&request, &succeeded)
                .expect("receipt")
                .is_succeeded()
        );
        let processing = serde_json::json!({
            "id": "pi_2",
            "amount": 2_500,
            "currency": "brl",
            "status": "processing"
        });
        assert_eq!(
            parse_response(&request, &processing)
                .expect("processing receipt")
                .status(),
            ChargeStatus::Processing
        );

        for invalid in [
            serde_json::json!({"id":"pi_1","amount":2_501,"amount_received":2_500,"currency":"brl","status":"succeeded"}),
            serde_json::json!({"id":"pi_1","amount":2_500,"amount_received":2_500,"currency":"usd","status":"succeeded"}),
            serde_json::json!({"id":"pi_1","amount":2_500,"amount_received":2_499,"currency":"brl","status":"succeeded"}),
            serde_json::json!({"id":"pi_1","amount":2_500,"currency":"brl","status":"requires_action"}),
            serde_json::json!({"amount":2_500,"amount_received":2_500,"currency":"brl","status":"succeeded"}),
        ] {
            assert!(parse_response(&request, &invalid).is_err());
        }

        let invalid_id = serde_json::json!({
            "id": "line\nbreak",
            "amount": 2_500,
            "amount_received": 2_500,
            "currency": "brl",
            "status": "succeeded"
        });
        assert!(matches!(
            parse_response(&request, &invalid_id),
            Err(CapitalError::ProviderRequestFailed(_))
        ));
    }

    #[test]
    fn outbound_request_carries_exact_money_identity_and_idempotency() {
        let request = request();
        let outbound =
            build_request(http_client(), "sk_test_fixture", &request).expect("outbound request");
        assert_eq!(outbound.method(), reqwest::Method::POST);
        assert_eq!(outbound.url().as_str(), PAYMENT_INTENTS_ENDPOINT);
        assert_eq!(
            outbound
                .headers()
                .get("idempotency-key")
                .and_then(|value| value.to_str().ok()),
            Some("order_1")
        );
        let body = outbound
            .body()
            .and_then(reqwest::Body::as_bytes)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .expect("form body");
        for field in [
            "amount=2500",
            "currency=brl",
            "customer=cus_1",
            "payment_method=pm_1",
            "receipt_email=a%40b.co",
            "confirm=true",
            "off_session=true",
            "error_on_requires_action=true",
        ] {
            assert!(body.contains(field), "missing `{field}` in `{body}`");
        }
    }

    #[tokio::test]
    async fn stripe_rejects_unscoped_provider_references_before_mock_or_network() {
        let invalid_customer =
            ChargeRequest::new(2_500, "BRL", "other_1", "a@b.co", "pm_1", "order_1")
                .expect("generic request");
        assert!(matches!(
            execute("mock_key", &invalid_customer).await,
            Err(CapitalError::InvalidCharge(_))
        ));

        let raw_number = ChargeRequest::new(
            2_500,
            "BRL",
            "cus_1",
            "a@b.co",
            "4242424242424242",
            "order_1",
        )
        .expect("generic request");
        assert!(matches!(
            execute("mock_key", &raw_number).await,
            Err(CapitalError::InvalidCharge(_))
        ));
    }
}
