use super::*;
use crate::{BillingProvider, OneOffPaymentState, StripeProvider};
use serde_json::json;

fn request() -> OneOffCheckoutRequest {
    OneOffCheckoutRequest::new(
        "order_42",
        2_500,
        "brl",
        "Group class & mat",
        "https://app.example/orders/42/paid?session={CHECKOUT_SESSION_ID}",
        "https://app.example/orders/42",
    )
    .unwrap()
    .with_quantity(3)
    .unwrap()
    .with_customer_email("buyer@example.com")
    .unwrap()
}

fn session() -> Value {
    json!({
        "id": "cs_test_42", "object": "checkout.session", "mode": "payment",
        "status": "open", "payment_status": "unpaid", "livemode": false,
        "client_reference_id": "order_42", "metadata": {"rullst_reference": "order_42"},
        "amount_total": 7_500, "currency": "brl",
        "url": "https://checkout.stripe.com/c/pay/cs_test_42#fragment"
    })
}

#[test]
fn form_sends_inline_price_data_quantity_and_no_customer() {
    let body = form(&request());
    let parsed = reqwest::Url::parse(&format!("https://fixture.invalid/?{body}")).unwrap();
    let fields = parsed
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(fields["mode"], "payment");
    assert_eq!(fields["line_items[0][quantity]"], "3");
    assert_eq!(fields["line_items[0][price_data][unit_amount]"], "2500");
    assert_eq!(fields["line_items[0][price_data][currency]"], "brl");
    assert_eq!(
        fields["line_items[0][price_data][product_data][name]"],
        "Group class & mat"
    );
    assert_eq!(fields["client_reference_id"], "order_42");
    assert_eq!(fields["metadata[rullst_reference]"], "order_42");
    assert_eq!(
        fields["payment_intent_data[metadata][rullst_reference]"],
        "order_42"
    );
    assert_eq!(fields["customer_email"], "buyer@example.com");
    assert_eq!(
        fields["success_url"],
        "https://app.example/orders/42/paid?session={CHECKOUT_SESSION_ID}"
    );
    assert_eq!(fields["allow_promotion_codes"], "false");
    assert!(!fields.contains_key("customer"));
    assert!(!fields.keys().any(|key| key.contains("[price]")));
    assert_eq!(fields.len(), 15);
    let without_email = OneOffCheckoutRequest::new(
        "order_42",
        2_500,
        "brl",
        "Class",
        "https://app.example/a",
        "https://app.example/b",
    )
    .unwrap();
    assert!(!form(&without_email).contains("customer_email"));
}

#[test]
fn created_session_must_match_the_request_and_key_mode() {
    let request = request();
    assert_eq!(
        validate_session("sk_test_x", &request, &session(), OPERATION).unwrap(),
        "cs_test_42"
    );
    for (field, value) in [
        ("id", json!("pi_42")),
        ("object", json!("payment_intent")),
        ("mode", json!("subscription")),
        ("client_reference_id", json!("order_43")),
        ("metadata", json!({"rullst_reference": "order_43"})),
        ("amount_total", json!(2_500)),
        ("currency", json!("usd")),
        ("livemode", json!(true)),
    ] {
        let mut changed = session();
        changed[field] = value;
        assert!(
            validate_session("sk_test_x", &request, &changed, OPERATION).is_err(),
            "{field}"
        );
    }
    let mut live = session();
    live["livemode"] = json!(true);
    validate_session("sk_live_x", &request, &live, OPERATION).unwrap();
}

#[test]
fn confirmation_is_paid_only_for_a_complete_paid_bound_session() {
    let request = request();
    let mut paid = session();
    paid["status"] = json!("complete");
    paid["payment_status"] = json!("paid");
    let payment = parse_confirmation("sk_test_x", "cs_test_42", &request, &paid).unwrap();
    assert_eq!(payment.state(), OneOffPaymentState::Paid);
    assert_eq!(payment.paid_amount_minor(), 7_500);
    assert_eq!(payment.transaction_id(), Some("cs_test_42"));

    for status in ["open", "complete", "expired"] {
        let mut unpaid = session();
        unpaid["status"] = json!(status);
        let payment = parse_confirmation("sk_test_x", "cs_test_42", &request, &unpaid).unwrap();
        assert_eq!(payment.state(), OneOffPaymentState::Unpaid, "{status}");
    }
    let mut odd = paid.clone();
    odd["payment_status"] = json!("no_payment_required");
    assert!(parse_confirmation("sk_test_x", "cs_test_42", &request, &odd).is_err());
    // The session read back must be the one the notification named.
    assert!(parse_confirmation("sk_test_x", "cs_test_other", &request, &paid).is_err());
    let mut cheaper = paid;
    cheaper["amount_total"] = json!(100);
    assert!(parse_confirmation("sk_test_x", "cs_test_42", &request, &cheaper).is_err());
}

#[tokio::test]
async fn mock_key_is_offline_and_never_paid() {
    let provider = StripeProvider::new("mock_key", "mock_webhook");
    let local = OneOffCheckoutRequest::new(
        "order_42",
        2_500,
        "brl",
        "Class",
        "http://localhost:3000/paid",
        "http://localhost:3000/cancel",
    )
    .unwrap();
    let checkout = provider.create_one_off_checkout(&local).await.unwrap();
    assert!(checkout.is_mock());
    assert!(checkout.url().starts_with("https://mock.stripe.invalid/"));
    let notification = PaymentNotification::new("order_42").unwrap();
    let payment = provider
        .confirm_one_off_payment(&local, &notification)
        .await
        .unwrap();
    assert_eq!(payment.state(), OneOffPaymentState::Mock);
    assert!(
        provider
            .confirm_one_off_payment(&local, &PaymentNotification::new("order_43").unwrap())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn live_key_refuses_loopback_redirects_and_unnamed_sessions_before_any_request() {
    let provider = StripeProvider::new("sk_test_fixture", "whsec_fixture");
    let local = OneOffCheckoutRequest::new(
        "order_42",
        2_500,
        "brl",
        "Class",
        "http://localhost:3000/paid",
        "https://app.example/cancel",
    )
    .unwrap();
    assert!(matches!(
        provider.create_one_off_checkout(&local).await,
        Err(CapitalError::ConfigurationError(message)) if message.contains("HTTPS")
    ));
    for notification in [
        PaymentNotification::new("order_42").unwrap(),
        PaymentNotification::new("order_42")
            .unwrap()
            .with_transaction_id("pi_not_a_session")
            .unwrap(),
    ] {
        assert!(matches!(
            provider
                .confirm_one_off_payment(&request(), &notification)
                .await,
            Err(CapitalError::PayloadParseError(_))
        ));
    }
}

mod fixture {
    use super::*;
    use axum::{
        Json, Router,
        extract::{Path, State},
        http::HeaderMap,
        routing::{get, post},
    };
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Stripe {
        created: Arc<Mutex<Value>>,
        read: Arc<Mutex<Value>>,
        seen: Arc<Mutex<Vec<(String, HeaderMap, String)>>>,
    }

    async fn read_session(
        State(stripe): State<Stripe>,
        Path(id): Path<String>,
        headers: HeaderMap,
    ) -> Json<Value> {
        stripe
            .seen
            .lock()
            .unwrap()
            .push((id, headers, String::new()));
        let reply = stripe.read.lock().unwrap().clone();
        Json(reply)
    }

    async fn serve(created: Value, read: Value) -> (String, Stripe) {
        let stripe = Stripe::default();
        *stripe.created.lock().unwrap() = created;
        *stripe.read.lock().unwrap() = read;
        let app = Router::new()
            .route(
                "/v1/checkout/sessions",
                post(
                    |State(stripe): State<Stripe>, headers: HeaderMap, body: String| async move {
                        stripe
                            .seen
                            .lock()
                            .unwrap()
                            .push(("create".into(), headers, body));
                        let reply = stripe.created.lock().unwrap().clone();
                        Json(reply)
                    },
                ),
            )
            // rullst-access: public — local Stripe fixture without data.
            .route("/v1/checkout/sessions/{id}", get(read_session))
            .with_state(stripe.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        (format!("http://{address}"), stripe)
    }

    fn paid() -> Value {
        let mut paid = session();
        paid["status"] = json!("complete");
        paid["payment_status"] = json!("paid");
        paid
    }

    #[tokio::test]
    async fn live_session_is_created_and_read_back_with_the_request_bindings() {
        let (base, stripe) = serve(session(), paid()).await;
        let request = request();
        let checkout = create_at(&base, "sk_test_x", &request).await.unwrap();
        assert_eq!(
            checkout.url(),
            "https://checkout.stripe.com/c/pay/cs_test_42#fragment"
        );
        assert_eq!(checkout.session_id(), Some("cs_test_42"));
        assert!(!checkout.is_mock());
        let notification = PaymentNotification::new("order_42")
            .unwrap()
            .with_transaction_id("cs_test_42")
            .unwrap();
        let payment = confirm_at(&base, "sk_test_x", &request, &notification)
            .await
            .unwrap();
        assert!(payment.is_paid());
        assert_eq!(payment.paid_amount_minor(), 7_500);

        let seen = stripe.seen.lock().unwrap();
        let (_, headers, body) = &seen[0];
        assert_eq!(headers["authorization"], "Bearer sk_test_x");
        assert_eq!(
            headers["idempotency-key"],
            request.idempotency_key().as_str()
        );
        assert_eq!(headers["stripe-version"], "2025-03-31.basil");
        assert_eq!(body, &form(&request));
        assert_eq!(seen[1].0, "cs_test_42");
        assert_eq!(seen[1].1["authorization"], "Bearer sk_test_x");
        assert_eq!(seen[1].1["stripe-version"], "2025-03-31.basil");
    }

    #[tokio::test]
    async fn a_created_session_must_be_open_unpaid_and_carry_a_url() {
        for (field, value) in [
            ("status", json!("complete")),
            ("payment_status", json!("paid")),
            ("url", Value::Null),
            ("url", json!("http://checkout.stripe.com/c/pay/cs_test_42")),
            ("amount_total", json!(2_500)),
        ] {
            let mut created = session();
            created[field] = value;
            let (base, _) = serve(created, paid()).await;
            assert!(
                matches!(
                    create_at(&base, "sk_test_x", &request()).await,
                    Err(CapitalError::Provider(_))
                ),
                "{field}"
            );
        }
    }

    #[tokio::test]
    async fn an_empty_key_is_the_offline_mock_too() {
        let (base, stripe) = serve(session(), paid()).await;
        assert!(create_at(&base, "", &request()).await.unwrap().is_mock());
        let notification = PaymentNotification::new("order_42").unwrap();
        assert_eq!(
            confirm_at(&base, "", &request(), &notification)
                .await
                .unwrap()
                .state(),
            OneOffPaymentState::Mock
        );
        assert!(stripe.seen.lock().unwrap().is_empty());
    }
}
