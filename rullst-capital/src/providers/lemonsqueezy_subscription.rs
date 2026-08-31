use super::http_client;
use crate::{
    CapitalError,
    subscription::{
        read_bounded_subscription_json, validate_provider_subscription_id, validate_trial_end,
    },
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
    let response = http_client()
        .patch(endpoint)
        .bearer_auth(api_key)
        .header("Accept", "application/vnd.api+json")
        .header("Content-Type", "application/vnd.api+json")
        .json(&request_body(subscription_id, &trial_ends_at_iso))
        .send()
        .await
        .map_err(|_| {
            CapitalError::ProviderRequestFailed(
                "Lemon Squeezy trial-extension transport failed".to_string(),
            )
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(CapitalError::ProviderRequestFailed(format!(
            "Lemon Squeezy trial-extension API returned HTTP {status}"
        )));
    }
    let body = read_bounded_subscription_json(response).await?;
    bind_response(subscription_id, trial_ends_at, &body)
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
        return Err(CapitalError::ProviderRequestFailed(
            "Lemon Squeezy trial response did not match the requested subscription and expiration"
                .to_string(),
        ));
    }
    Ok(())
}

#[cfg(all(test, feature = "axum"))]
mod tests {
    use super::*;
    use axum::{Json, Router, extract::State, http::HeaderMap, routing::patch};
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

    #[tokio::test]
    async fn trial_update_uses_json_api_and_binds_response() {
        let (endpoint, mut receiver, server) = start_fixture().await;
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
        (
            format!("http://{address}/v1/subscriptions/42"),
            receiver,
            server,
        )
    }
}
