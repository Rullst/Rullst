//! Live Razorpay subscription pause.
//!
//! Razorpay's "Pause a Subscription" reference
//! (<https://razorpay.com/docs/api/payments/subscriptions/pause-subscription/>)
//! documents `POST /v1/subscriptions/{id}/pause` with the JSON body
//! `{"pause_at": "now"}`; `now` is the only documented value. The response is
//! the subscription entity with `status` `paused`. Only an `active`
//! subscription can be paused: Razorpay moves an `authenticated` one to
//! `cancelled` instead, which is reported as an error here.

use super::{http_client, send_http_json};
use crate::{CapitalError, ProviderFailure, subscription::validate_provider_subscription_id};
use serde_json::{Value, json};

const RAZORPAY_SUBSCRIPTIONS_ENDPOINT: &str = "https://api.razorpay.com/v1/subscriptions";
const OPERATION: &str = "pause subscription";

pub(super) async fn pause(
    key_id: &str,
    key_secret: &str,
    subscription_id: &str,
) -> Result<(), CapitalError> {
    validate_provider_subscription_id(subscription_id)?;
    let endpoint = format!("{RAZORPAY_SUBSCRIPTIONS_ENDPOINT}/{subscription_id}/pause");
    pause_at(key_id, key_secret, &endpoint, subscription_id).await
}

async fn pause_at(
    key_id: &str,
    key_secret: &str,
    endpoint: &str,
    subscription_id: &str,
) -> Result<(), CapitalError> {
    let body: Value = send_http_json(
        http_client()?
            .post(endpoint)
            .basic_auth(key_id, Some(key_secret))
            .json(&json!({ "pause_at": "now" })),
        "razorpay",
        OPERATION,
    )
    .await?;
    bind_pause_response(subscription_id, &body)
}

fn bind_pause_response(subscription_id: &str, body: &Value) -> Result<(), CapitalError> {
    if body["entity"].as_str() != Some("subscription")
        || body["id"].as_str() != Some(subscription_id)
    {
        return Err(ProviderFailure::contract_mismatch("razorpay", OPERATION).into());
    }
    match body["status"].as_str() {
        Some("paused") => Ok(()),
        Some("cancelled") => Err(CapitalError::SubscriptionError(
            "Razorpay cancelled the subscription instead of pausing it; only an active subscription can be paused"
                .to_string(),
        )),
        _ => Err(ProviderFailure::contract_mismatch("razorpay", OPERATION).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderFailureKind;
    use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
    use tokio::sync::mpsc;

    #[derive(Debug)]
    struct CapturedRequest {
        authorization: Option<String>,
        content_type: Option<String>,
        body: Value,
    }

    type Sender = mpsc::UnboundedSender<CapturedRequest>;

    async fn capture(sender: &Sender, headers: &HeaderMap, body: Value) {
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        };
        sender
            .send(CapturedRequest {
                authorization: header("authorization"),
                content_type: header("content-type"),
                body,
            })
            .expect("capture receiver remains open");
    }

    async fn paused(
        State(sender): State<Sender>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        capture(&sender, &headers, body).await;
        Json(json!({
            "id": "sub_live",
            "entity": "subscription",
            "status": "paused",
            "paused_at": 1_590_280_581,
            "pause_initiated_by": "self"
        }))
    }

    async fn cancelled(State(sender): State<Sender>, headers: HeaderMap) -> Json<Value> {
        capture(&sender, &headers, Value::Null).await;
        Json(json!({"id": "sub_authenticated", "entity": "subscription", "status": "cancelled"}))
    }

    async fn other(State(sender): State<Sender>, headers: HeaderMap) -> Json<Value> {
        capture(&sender, &headers, Value::Null).await;
        Json(json!({"id": "sub_somebody_else", "entity": "subscription", "status": "paused"}))
    }

    async fn start_fixture() -> (
        String,
        mpsc::UnboundedReceiver<CapturedRequest>,
        tokio::task::JoinHandle<()>,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let router = Router::new()
            .route("/v1/subscriptions/sub_live/pause", post(paused))
            .route("/v1/subscriptions/sub_authenticated/pause", post(cancelled))
            .route("/v1/subscriptions/sub_other/pause", post(other))
            .with_state(sender);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind Razorpay fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("serve Razorpay fixture");
        });
        (
            format!("http://{address}/v1/subscriptions"),
            receiver,
            server,
        )
    }

    #[tokio::test]
    async fn live_pause_sends_pause_at_now_and_binds_the_paused_subscription() {
        let (base, mut receiver, server) = start_fixture().await;
        pause_at(
            "rzp_fixture",
            "fixture_secret",
            &format!("{base}/sub_live/pause"),
            "sub_live",
        )
        .await
        .expect("paused subscription");
        let request = receiver.recv().await.expect("captured pause");
        assert_eq!(request.body, json!({"pause_at": "now"}));
        assert_eq!(request.content_type.as_deref(), Some("application/json"));
        assert_eq!(
            request.authorization.as_deref(),
            Some("Basic cnpwX2ZpeHR1cmU6Zml4dHVyZV9zZWNyZXQ=")
        );

        let cancelled = pause_at(
            "rzp_fixture",
            "fixture_secret",
            &format!("{base}/sub_authenticated/pause"),
            "sub_authenticated",
        )
        .await;
        assert!(matches!(cancelled, Err(CapitalError::SubscriptionError(_))));

        let mismatch = pause_at(
            "rzp_fixture",
            "fixture_secret",
            &format!("{base}/sub_other/pause"),
            "sub_other",
        )
        .await;
        assert!(matches!(
            mismatch,
            Err(CapitalError::Provider(failure))
                if failure.kind() == ProviderFailureKind::ContractMismatch
        ));
        server.abort();
    }

    #[test]
    fn pause_response_must_be_the_requested_paused_subscription() {
        let paused = json!({"id": "sub_1", "entity": "subscription", "status": "paused"});
        assert!(bind_pause_response("sub_1", &paused).is_ok());
        assert!(bind_pause_response("sub_2", &paused).is_err());
        for body in [
            json!({"id": "sub_1", "entity": "plan", "status": "paused"}),
            json!({"id": "sub_1", "entity": "subscription", "status": "active"}),
            json!({"id": "sub_1", "entity": "subscription"}),
        ] {
            assert!(bind_pause_response("sub_1", &body).is_err());
        }
    }
}
