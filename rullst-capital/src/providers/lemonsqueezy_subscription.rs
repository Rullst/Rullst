use super::{http_client, send_http, send_http_json};
use crate::{
    CapitalError,
    subscription::{validate_provider_subscription_id, validate_trial_end},
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Value, json};

const LEMON_SUBSCRIPTIONS_ENDPOINT: &str = "https://api.lemonsqueezy.com/v1/subscriptions";

pub(super) async fn extend_trial(
    api_key: &str,
    subscription_id: &str,
    trial_ends_at: i64,
) -> Result<(), CapitalError> {
    validate_provider_subscription_id(subscription_id)?;
    let endpoint = format!("{LEMON_SUBSCRIPTIONS_ENDPOINT}/{subscription_id}");
    extend_trial_at(api_key, &endpoint, subscription_id, trial_ends_at).await
}

async fn extend_trial_at(
    api_key: &str,
    endpoint: &str,
    subscription_id: &str,
    trial_ends_at: i64,
) -> Result<(), CapitalError> {
    validate_provider_subscription_id(subscription_id)?;
    validate_trial_end(trial_ends_at)?;
    let trial_ends_at_iso = DateTime::<Utc>::from_timestamp(trial_ends_at, 0)
        .ok_or_else(|| {
            CapitalError::SubscriptionError(
                "trial end timestamp is outside the supported UTC range".to_string(),
            )
        })?
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    if api_key.is_empty() || api_key.starts_with("mock_") {
        return Ok(());
    }
    let body: Value = send_http_json(
        http_client()?
            .patch(endpoint)
            .bearer_auth(api_key)
            .header("Accept", "application/vnd.api+json")
            .header("Content-Type", "application/vnd.api+json")
            .json(&request_body(subscription_id, &trial_ends_at_iso)),
        "lemonsqueezy",
        "extend trial",
    )
    .await?;
    bind_response(subscription_id, trial_ends_at, &body)
}

// Every Lemon Squeezy JSON:API call carries its media type, including the
// legacy pause and cancel operations.
const JSON_API: &str = "application/vnd.api+json";

pub(super) async fn pause(api_key: &str, subscription_id: &str) -> Result<(), CapitalError> {
    validate_provider_subscription_id(subscription_id)?;
    let endpoint = format!("{LEMON_SUBSCRIPTIONS_ENDPOINT}/{subscription_id}");
    pause_at(api_key, &endpoint, subscription_id).await
}

async fn pause_at(
    api_key: &str,
    endpoint: &str,
    subscription_id: &str,
) -> Result<(), CapitalError> {
    let payload = json!({
        "data": {
            "type": "subscriptions",
            "id": subscription_id,
            "attributes": {
                "pause": {
                    "mode": "void"
                }
            }
        }
    });
    send_http(
        http_client()?
            .patch(endpoint)
            .bearer_auth(api_key)
            .header("Accept", JSON_API)
            .header("Content-Type", JSON_API)
            .json(&payload),
        "lemonsqueezy",
        "pause subscription",
    )
    .await?;
    Ok(())
}

pub(super) async fn cancel(api_key: &str, subscription_id: &str) -> Result<(), CapitalError> {
    validate_provider_subscription_id(subscription_id)?;
    cancel_at(
        api_key,
        &format!("{LEMON_SUBSCRIPTIONS_ENDPOINT}/{subscription_id}"),
    )
    .await
}

async fn cancel_at(api_key: &str, endpoint: &str) -> Result<(), CapitalError> {
    send_http(
        http_client()?
            .delete(endpoint)
            .bearer_auth(api_key)
            .header("Accept", JSON_API)
            .header("Content-Type", JSON_API),
        "lemonsqueezy",
        "cancel subscription",
    )
    .await?;
    Ok(())
}

fn request_body(subscription_id: &str, trial_ends_at: &str) -> Value {
    json!({
        "data": {
            "type": "subscriptions",
            "id": subscription_id,
            "attributes": {
                "trial_ends_at": trial_ends_at
            }
        }
    })
}

fn bind_response(
    subscription_id: &str,
    trial_ends_at: i64,
    body: &Value,
) -> Result<(), CapitalError> {
    let data = body.get("data");
    let response_id = data.and_then(|value| value.get("id"));
    let id_matches = response_id.and_then(Value::as_str) == Some(subscription_id)
        || response_id
            .and_then(Value::as_u64)
            .is_some_and(|value| value.to_string() == subscription_id);
    let type_matches = data
        .and_then(|value| value.get("type"))
        .and_then(Value::as_str)
        == Some("subscriptions");
    let response_end = data
        .and_then(|value| value.get("attributes"))
        .and_then(|value| value.get("trial_ends_at"))
        .and_then(Value::as_str)
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.timestamp());
    if !id_matches || !type_matches || response_end != Some(trial_ends_at) {
        return Err(
            crate::ProviderFailure::contract_mismatch("lemonsqueezy", "extend trial").into(),
        );
    }
    Ok(())
}

#[cfg(all(test, feature = "axum"))]
mod tests {
    use super::*;
    use axum::{Json, Router, body::Bytes, extract::State, http::HeaderMap, routing::patch};
    use tokio::sync::mpsc;

    #[derive(Debug)]
    struct CapturedRequest {
        authorization: Option<String>,
        accept: Option<String>,
        content_type: Option<String>,
        body: Value,
    }

    async fn fixture(
        State(sender): State<mpsc::UnboundedSender<CapturedRequest>>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        sender
            .send(CapturedRequest {
                authorization: header(&headers, "authorization"),
                accept: header(&headers, "accept"),
                content_type: header(&headers, "content-type"),
                body: body.clone(),
            })
            .expect("capture receiver remains open");
        Json(json!({
            "data": {
                "type": "subscriptions",
                "id": "42",
                "attributes": {
                    "trial_ends_at": body.pointer("/data/attributes/trial_ends_at")
                }
            }
        }))
    }

    async fn capture(
        State(sender): State<mpsc::UnboundedSender<CapturedRequest>>,
        headers: HeaderMap,
        body: Bytes,
    ) -> Json<Value> {
        sender
            .send(CapturedRequest {
                authorization: header(&headers, "authorization"),
                accept: header(&headers, "accept"),
                content_type: header(&headers, "content-type"),
                body: serde_json::from_slice(&body).unwrap_or(Value::Null),
            })
            .expect("capture receiver remains open");
        Json(json!({"data": {"type": "subscriptions", "id": "43"}}))
    }

    #[tokio::test]
    async fn pause_and_cancel_send_json_api_media_types() {
        let (base, mut receiver, server) = start_fixture().await;
        let endpoint = format!("{base}/v1/subscriptions/43");
        pause_at("lemon_fixture", &endpoint, "43")
            .await
            .expect("pause");
        let request = receiver.recv().await.expect("captured pause");
        assert_eq!(request.accept.as_deref(), Some(JSON_API));
        assert_eq!(request.content_type.as_deref(), Some(JSON_API));
        assert_eq!(
            request.body.pointer("/data/attributes/pause/mode"),
            Some(&json!("void"))
        );

        cancel_at("lemon_fixture", &endpoint).await.expect("cancel");
        let request = receiver.recv().await.expect("captured cancel");
        assert_eq!(
            request.authorization.as_deref(),
            Some("Bearer lemon_fixture")
        );
        assert_eq!(request.accept.as_deref(), Some(JSON_API));
        server.abort();
    }

    #[tokio::test]
    async fn trial_update_uses_json_api_and_binds_response() {
        let (base, mut receiver, server) = start_fixture().await;
        let endpoint = format!("{base}/v1/subscriptions/42");
        extend_trial_at("lemon_fixture", &endpoint, "42", 1_900_000_000)
            .await
            .expect("trial update");
        let request = receiver.recv().await.expect("captured request");
        assert_eq!(
            request.authorization.as_deref(),
            Some("Bearer lemon_fixture")
        );
        assert_eq!(request.accept.as_deref(), Some("application/vnd.api+json"));
        assert_eq!(
            request.content_type.as_deref(),
            Some("application/vnd.api+json")
        );
        assert_eq!(request.body.pointer("/data/id"), Some(&json!("42")));
        assert_eq!(
            request.body.pointer("/data/attributes/trial_ends_at"),
            Some(&json!("2030-03-17T17:46:40Z"))
        );
        for mismatched in [
            json!({
                "data": {"type": "subscriptions", "id": "43", "attributes": {
                    "trial_ends_at": "2030-03-17T17:46:40Z"
                }}
            }),
            json!({
                "data": {"type": "subscriptions", "id": "42", "attributes": {
                    "trial_ends_at": "2030-03-18T17:46:40Z"
                }}
            }),
        ] {
            assert!(bind_response("42", 1_900_000_000, &mismatched).is_err());
        }
        assert!(
            extend_trial_at("lemon_fixture", &endpoint, "../42", 1)
                .await
                .is_err()
        );
        assert!(
            extend_trial_at("lemon_fixture", &endpoint, "42", 0)
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
            .route("/v1/subscriptions/42", patch(fixture))
            .route("/v1/subscriptions/43", patch(capture).delete(capture))
            .with_state(sender);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind Lemon fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("serve Lemon fixture");
        });
        (format!("http://{address}"), receiver, server)
    }
}
