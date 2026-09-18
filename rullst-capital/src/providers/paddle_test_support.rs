use super::PaddleProvider;
use crate::PaddleCheckoutRequest;
use axum::{Router, body::to_bytes, response::IntoResponse, routing::any};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub(super) const CUSTOMER: &str = "ctm_01hv6y1jedq4p1n0yqn5ba3ky4";
pub(super) const PRICE: &str = "pri_01gsz8x8sawmvhz1pv30nge1ke";
pub(super) const TRANSACTION: &str = "txn_01hv8m0mnx3sj85e7gxc6kga03";
pub(super) const SUBSCRIPTION: &str = "sub_01hv8x29kz0t586xy6zn1a62ny";
pub(super) const EVENT: &str = "evt_01hv8x2af22vrrz7k67g06x1kq";
pub(super) const SECRET: &str = "pdl_ntfset_test_contract_secret";
pub(super) fn request() -> PaddleCheckoutRequest {
    PaddleCheckoutRequest::new(
        CUSTOMER,
        PRICE,
        "owner_opaque",
        "attempt_opaque",
        "https://app.example/pay",
    )
    .unwrap()
}
pub(super) fn customer() -> Value {
    json!({"id": CUSTOMER, "email": "contact@example.test", "status": "active", "custom_data": {"rullst_owner_reference":"owner_opaque", "rullst_attempt_reference":"provision_opaque"}})
}
pub(super) fn items() -> Value {
    json!([{"price": {"id": PRICE, "billing_cycle": {"interval":"month", "frequency":1}}, "quantity":1}])
}
pub(super) fn transaction() -> Value {
    json!({"id": TRANSACTION, "status":"draft", "customer_id":CUSTOMER, "origin":"api", "collection_mode":"automatic",
        "items": items(), "custom_data":{"rullst_owner_reference":"owner_opaque", "rullst_attempt_reference":"attempt_opaque"},
        "checkout":{"url":format!("https://app.example/pay?_ptxn={TRANSACTION}")}, "subscription_id":null})
}
pub(super) fn subscription() -> Value {
    json!({"id":SUBSCRIPTION, "customer_id":CUSTOMER, "status":"active", "collection_mode":"automatic", "items": items(),
        "custom_data":{"rullst_owner_reference":"owner_opaque", "rullst_attempt_reference":"attempt_opaque"},
        "transaction_id":TRANSACTION, "current_billing_period":{"starts_at":"2026-09-01T00:00:00Z", "ends_at":"2026-10-01T00:00:00Z"}})
}
pub(super) struct Fixture {
    pub provider: PaddleProvider,
    pub requests: Arc<Mutex<Vec<(String, String, Value)>>>,
    pub responses: Arc<Mutex<HashMap<String, (u16, Value)>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Fixture {
    pub fn respond(&self, method: &str, path: &str, status: u16, data: Value) {
        self.responses
            .lock()
            .unwrap()
            .insert(format!("{method} {path}"), (status, json!({"data":data})));
    }
}
pub(super) async fn fixture() -> Fixture {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses: Arc<Mutex<HashMap<String, (u16, Value)>>> = Arc::new(Mutex::new(HashMap::new()));
    let seen = requests.clone();
    let replies = responses.clone();
    let app = Router::new().fallback(any(move |request: axum::extract::Request| {
        let seen = seen.clone();
        let replies = replies.clone();
        async move {
            assert_eq!(request.headers()["paddle-version"], "1");
            assert_eq!(request.headers()["authorization"], "Bearer fixture_key");
            assert!(!request.headers().contains_key("idempotency-key"));
            let method = request.method().to_string();
            let path = request.uri().path().to_string();
            let body = to_bytes(request.into_body(), 8192).await.unwrap();
            let body = if body.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(&body).unwrap()
            };
            seen.lock()
                .unwrap()
                .push((method.clone(), path.clone(), body));
            let response = replies
                .lock()
                .unwrap()
                .get(&format!("{method} {path}"))
                .cloned()
                .unwrap_or((404, json!({})));
            (
                axum::http::StatusCode::from_u16(response.0).unwrap(),
                axum::Json(response.1),
            )
                .into_response()
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let mut provider = PaddleProvider::new("fixture_key", SECRET).with_sandbox(true);
    provider.fixture_url = Some(format!("http://{address}"));
    let fixture = Fixture {
        provider,
        requests,
        responses,
        task,
    };
    fixture.respond("POST", "/customers", 201, customer());
    fixture.respond("GET", &format!("/customers/{CUSTOMER}"), 200, customer());
    fixture.respond("POST", "/transactions", 201, transaction());
    fixture.respond(
        "GET",
        &format!("/transactions/{TRANSACTION}"),
        200,
        transaction(),
    );
    fixture.respond(
        "GET",
        &format!("/subscriptions/{SUBSCRIPTION}"),
        200,
        subscription(),
    );
    fixture
}
