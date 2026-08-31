use super::{http_client, url_encode};
use crate::{
    CapitalError,
    subscription::{
        read_bounded_subscription_json, validate_coupon_code, validate_provider_subscription_id,
        validate_trial_end,
    },
};
use serde_json::Value;

const STRIPE_SUBSCRIPTIONS_ENDPOINT: &str = "https://api.stripe.com/v1/subscriptions";

pub(super) async fn apply_coupon(
    api_key: &str,
    subscription_id: &str,
    coupon_code: &str,
) -> Result<(), CapitalError> {
    let endpoint = subscription_endpoint(STRIPE_SUBSCRIPTIONS_ENDPOINT, subscription_id)?;
    apply_coupon_at(api_key, &endpoint, subscription_id, coupon_code).await
}

pub(super) async fn extend_trial(
    api_key: &str,
    subscription_id: &str,
    trial_ends_at: i64,
) -> Result<(), CapitalError> {
    let endpoint = subscription_endpoint(STRIPE_SUBSCRIPTIONS_ENDPOINT, subscription_id)?;
    extend_trial_at(api_key, &endpoint, subscription_id, trial_ends_at).await
}

async fn apply_coupon_at(
    api_key: &str,
    endpoint: &str,
    subscription_id: &str,
    coupon_code: &str,
) -> Result<(), CapitalError> {
    validate_provider_subscription_id(subscription_id)?;
    let coupon = validate_coupon_code(coupon_code)?;
    if api_key.is_empty() || api_key.starts_with("mock_") {
        return Ok(());
    }
    let body = format!(
        "discounts%5B0%5D%5Bcoupon%5D={}&expand%5B%5D=discounts",
        url_encode(coupon.as_str())
    );
    let response = send_form(api_key, endpoint, body, "discount").await?;
    let body = read_bounded_subscription_json(response).await?;
    bind_coupon_response(subscription_id, coupon.as_str(), &body)
}

fn bind_coupon_response(
    subscription_id: &str,
    coupon_code: &str,
    body: &Value,
) -> Result<(), CapitalError> {
    bind_subscription_id(subscription_id, body, "discount")?;
    let coupon_matches = body
        .get("discounts")
        .and_then(Value::as_array)
        .is_some_and(|discounts| {
            discounts.iter().any(|discount| {
                (discount.pointer("/source/type").and_then(Value::as_str) == Some("coupon")
                    && discount.pointer("/source/coupon").and_then(Value::as_str)
                        == Some(coupon_code))
                    || discount.pointer("/coupon/id").and_then(Value::as_str) == Some(coupon_code)
            })
        });
    if !coupon_matches {
        return Err(CapitalError::ProviderRequestFailed(
            "Stripe discount response did not contain the requested coupon".to_string(),
        ));
    }
    Ok(())
}

async fn extend_trial_at(
    api_key: &str,
    endpoint: &str,
    subscription_id: &str,
    trial_ends_at: i64,
) -> Result<(), CapitalError> {
    validate_provider_subscription_id(subscription_id)?;
    validate_trial_end(trial_ends_at)?;
    if api_key.is_empty() || api_key.starts_with("mock_") {
        return Ok(());
    }
    let response = send_form(
        api_key,
        endpoint,
        format!("trial_end={trial_ends_at}"),
        "trial extension",
    )
    .await?;
    let body = read_bounded_subscription_json(response).await?;
    bind_subscription_id(subscription_id, &body, "trial extension")?;
    if body.get("trial_end").and_then(Value::as_i64) != Some(trial_ends_at) {
        return Err(CapitalError::ProviderRequestFailed(
            "Stripe trial response did not match the requested expiration".to_string(),
        ));
    }
    Ok(())
}

async fn send_form(
    api_key: &str,
    endpoint: &str,
    body: String,
    operation: &str,
) -> Result<reqwest::Response, CapitalError> {
    let response = http_client()
        .post(endpoint)
        .bearer_auth(api_key)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|_| {
            CapitalError::ProviderRequestFailed(format!(
                "Stripe subscription {operation} transport failed"
            ))
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(CapitalError::ProviderRequestFailed(format!(
            "Stripe subscription {operation} returned HTTP {status}"
        )));
    }
    Ok(response)
}

fn subscription_endpoint(base: &str, subscription_id: &str) -> Result<String, CapitalError> {
    validate_provider_subscription_id(subscription_id)?;
    Ok(format!("{base}/{}", url_encode(subscription_id)))
}

fn bind_subscription_id(
    subscription_id: &str,
    body: &Value,
    operation: &str,
) -> Result<(), CapitalError> {
    if body.get("id").and_then(Value::as_str) != Some(subscription_id)
        || body.get("object").and_then(Value::as_str) != Some("subscription")
    {
        return Err(CapitalError::ProviderRequestFailed(format!(
            "Stripe {operation} response did not match the requested subscription"
        )));
    }
    Ok(())
}

#[cfg(all(test, feature = "axum"))]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        body::Bytes,
        extract::{Path, State},
        http::HeaderMap,
        routing::post,
    };
    use tokio::sync::mpsc;

    #[derive(Debug)]
    struct CapturedRequest {
        id: String,
        authorization: Option<String>,
        content_type: Option<String>,
        body: String,
    }

    async fn fixture(
        State(sender): State<mpsc::UnboundedSender<CapturedRequest>>,
        Path(id): Path<String>,
        headers: HeaderMap,
        body: Bytes,
    ) -> Json<Value> {
        let body = String::from_utf8_lossy(&body).into_owned();
        sender
            .send(CapturedRequest {
                id: id.clone(),
                authorization: header(&headers, "authorization"),
                content_type: header(&headers, "content-type"),
                body: body.clone(),
            })
            .expect("capture receiver remains open");
        let trial_end = body
            .strip_prefix("trial_end=")
            .and_then(|value| value.parse::<i64>().ok());
        let coupon = body
            .strip_prefix("discounts%5B0%5D%5Bcoupon%5D=")
            .and_then(|value| value.split('&').next());
        let padding = (coupon == Some("OVERSIZED")).then(|| "x".repeat(1024 * 1024));
        Json(serde_json::json!({
            "id": id,
            "object": "subscription",
            "trial_end": trial_end,
            "padding": padding,
            "discounts": coupon.map(|coupon| serde_json::json!({
                "id": "di_fixture",
                "object": "discount",
                "source": {"type": "coupon", "coupon": coupon}
            })).into_iter().collect::<Vec<_>>()
        }))
    }

    #[tokio::test]
    async fn discount_and_trial_protocols_are_exact_and_bound() {
        let (endpoint, mut receiver, server) = start_fixture().await;
        apply_coupon_at("sk_fixture", &endpoint, "sub_123", "BLACK_FRIDAY")
            .await
            .expect("coupon update");
        let coupon = receiver.recv().await.expect("coupon request");
        assert_eq!(coupon.id, "sub_123");
        assert_eq!(coupon.authorization.as_deref(), Some("Bearer sk_fixture"));
        assert_eq!(
            coupon.content_type.as_deref(),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(
            coupon.body,
            "discounts%5B0%5D%5Bcoupon%5D=BLACK_FRIDAY&expand%5B%5D=discounts"
        );

        extend_trial_at("sk_fixture", &endpoint, "sub_123", 1_900_000_000)
            .await
            .expect("trial update");
        let trial = receiver.recv().await.expect("trial request");
        assert_eq!(trial.body, "trial_end=1900000000");

        let mismatched = serde_json::json!({
            "id": "sub_other",
            "object": "subscription",
            "discounts": [{"source": {"type": "coupon", "coupon": "BLACK_FRIDAY"}}]
        });
        assert!(bind_coupon_response("sub_123", "BLACK_FRIDAY", &mismatched).is_err());
        let wrong_coupon = serde_json::json!({
            "id": "sub_123",
            "object": "subscription",
            "discounts": [{"source": {"type": "coupon", "coupon": "OTHER"}}]
        });
        assert!(bind_coupon_response("sub_123", "BLACK_FRIDAY", &wrong_coupon).is_err());

        let oversized = apply_coupon_at("sk_fixture", &endpoint, "sub_123", "OVERSIZED")
            .await
            .expect_err("oversized response must fail closed");
        assert!(oversized.to_string().contains("exceeded 1 MiB"));
        let _ = receiver.recv().await.expect("oversized request");

        assert!(
            apply_coupon_at("sk_fixture", &endpoint, "../sub", "CODE")
                .await
                .is_err()
        );
        assert!(
            apply_coupon_at("sk_fixture", &endpoint, "sub_123", "bad code")
                .await
                .is_err()
        );
        assert!(
            extend_trial_at("sk_fixture", &endpoint, "sub_123", 0)
                .await
                .is_err()
        );
        server.abort();
    }

    fn header(headers: &HeaderMap, name: &str) -> Option<String> {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    async fn start_fixture() -> (
        String,
        mpsc::UnboundedReceiver<CapturedRequest>,
        tokio::task::JoinHandle<()>,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let router = Router::new()
            .route("/v1/subscriptions/{id}", post(fixture))
            .with_state(sender);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind Stripe fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("serve Stripe fixture");
        });
        (
            format!("http://{address}/v1/subscriptions/sub_123"),
            receiver,
            server,
        )
    }
}
