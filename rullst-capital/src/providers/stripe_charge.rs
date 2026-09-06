use super::{execute_http, http_client, read_http_json, url_encode};
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

    let client = http_client()?;
    let outbound = build_request(client, api_key, request)?;
    let response = execute_http(outbound, "stripe", "direct charge").await?;
    let response: Value = read_http_json(response, "stripe", "direct charge").await?;
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
        .map_err(|_| crate::ProviderFailure::request_build("stripe", "direct charge").into())
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
    let charge_id = response["id"].as_str().ok_or_else(charge_mismatch)?;
    let amount_minor = response["amount"].as_u64().ok_or_else(charge_mismatch)?;
    let currency = response["currency"].as_str().ok_or_else(charge_mismatch)?;
    if amount_minor != request.amount_minor()
        || currency != request.currency()
        || !charge_id.starts_with("pi_")
        || response["object"].as_str() != Some("payment_intent")
        || response["customer"].as_str() != Some(request.customer_id())
        || response["payment_method"].as_str() != Some(request.payment_method_id())
        || response["receipt_email"].as_str() != Some(request.customer_email())
    {
        return Err(charge_mismatch());
    }
    let status = match response["status"].as_str() {
        Some("succeeded") => ChargeStatus::Succeeded,
        Some("processing") => ChargeStatus::Processing,
        Some(_) | None => return Err(charge_mismatch()),
    };
    if status == ChargeStatus::Succeeded {
        let amount_received = response["amount_received"]
            .as_u64()
            .ok_or_else(charge_mismatch)?;
        if amount_received != request.amount_minor() {
            return Err(charge_mismatch());
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
    .map_err(|_| charge_mismatch())
}

fn charge_mismatch() -> CapitalError {
    crate::ProviderFailure::contract_mismatch("stripe", "direct charge").into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ChargeRequest {
        ChargeRequest::new(2_500, "BRL", "cus_1", "a@b.co", "pm_1", "order_1").expect("request")
    }

    #[test]
    fn charge_response_rejects_cross_customer_and_payment_method_evidence() {
        let request = request();
        let valid = serde_json::json!({
            "id":"pi_1", "object":"payment_intent", "amount":2500,
            "amount_received":2500, "currency":"brl", "status":"succeeded",
            "customer":"cus_1", "payment_method":"pm_1", "receipt_email":"a@b.co"
        });
        assert!(parse_response(&request, &valid).is_ok());
        for (field, value) in [
            ("customer", serde_json::json!("cus_other")),
            ("payment_method", serde_json::json!("pm_other")),
            ("receipt_email", serde_json::json!("other@example.invalid")),
            ("object", serde_json::json!("refund")),
            ("id", serde_json::json!("re_1")),
            ("customer", serde_json::Value::Null),
            ("payment_method", serde_json::Value::Null),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert!(
                parse_response(&request, &invalid).is_err(),
                "accepted mismatched {field}"
            );
        }
    }

    #[test]
    fn response_is_bound_to_request_and_closed_to_non_accepted_statuses() {
        let request = request();
        let succeeded = serde_json::json!({
            "object":"payment_intent", "customer":"cus_1", "payment_method":"pm_1", "receipt_email":"a@b.co",
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
            "object":"payment_intent", "customer":"cus_1", "payment_method":"pm_1", "receipt_email":"a@b.co",
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

        for mut invalid in [
            serde_json::json!({"id":"pi_1","amount":2_501,"amount_received":2_500,"currency":"brl","status":"succeeded"}),
            serde_json::json!({"id":"pi_1","amount":2_500,"amount_received":2_500,"currency":"usd","status":"succeeded"}),
            serde_json::json!({"id":"pi_1","amount":2_500,"amount_received":2_499,"currency":"brl","status":"succeeded"}),
            serde_json::json!({"id":"pi_1","amount":2_500,"currency":"brl","status":"requires_action"}),
            serde_json::json!({"amount":2_500,"amount_received":2_500,"currency":"brl","status":"succeeded"}),
        ] {
            for field in ["object", "customer", "payment_method", "receipt_email"] {
                invalid[field] = succeeded[field].clone();
            }
            assert!(parse_response(&request, &invalid).is_err());
        }

        let invalid_id = serde_json::json!({
            "object":"payment_intent", "customer":"cus_1", "payment_method":"pm_1", "receipt_email":"a@b.co",
            "id": "line\nbreak",
            "amount": 2_500,
            "amount_received": 2_500,
            "currency": "brl",
            "status": "succeeded"
        });
        assert!(matches!(
            parse_response(&request, &invalid_id),
            Err(CapitalError::Provider(failure))
                if failure.kind() == crate::ProviderFailureKind::ContractMismatch
        ));
    }

    #[test]
    fn outbound_request_carries_exact_money_identity_and_idempotency() {
        let request = request();
        let outbound = build_request(
            http_client().expect("bounded client"),
            "sk_test_fixture",
            &request,
        )
        .expect("outbound request");
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
