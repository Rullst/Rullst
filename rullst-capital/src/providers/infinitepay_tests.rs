use super::*;
use crate::{OneOffPaymentState, StripeProvider};
use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use serde_json::json;
use std::sync::{Arc, Mutex};

/// Path, `X-Api-Key` header and JSON body of each request.
type Seen = Arc<Mutex<Vec<(String, Option<String>, Value)>>>;

#[derive(Clone, Default)]
struct Fixture {
    seen: Seen,
    check: Arc<Mutex<Value>>,
    link: Arc<Mutex<Value>>,
}

async fn record(fixture: &Fixture, path: &str, headers: &HeaderMap, body: Value) {
    let key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    fixture.seen.lock().unwrap().push((path.into(), key, body));
}

async fn api() -> (String, Fixture) {
    let fixture = Fixture::default();
    *fixture.link.lock().unwrap() =
        json!({ "url": "https://checkout.infinitepay.io/loja?lenc=abc" });
    *fixture.check.lock().unwrap() = json!({
        "success": true, "paid": true, "amount": 15_000, "paid_amount": 15_600,
        "installments": 2, "capture_method": "credit_card"
    });
    let app =
        Router::new()
            .route(
                "/links",
                post(
                    |State(fixture): State<Fixture>,
                     headers: HeaderMap,
                     Json(body): Json<Value>| async move {
                        record(&fixture, "/links", &headers, body).await;
                        let reply = fixture.link.lock().unwrap().clone();
                        Json(reply)
                    },
                ),
            )
            .route(
                "/payment_check",
                post(
                    |State(fixture): State<Fixture>,
                     headers: HeaderMap,
                     Json(body): Json<Value>| async move {
                        record(&fixture, "/payment_check", &headers, body).await;
                        let reply = fixture.check.lock().unwrap().clone();
                        Json(reply)
                    },
                ),
            )
            .with_state(fixture.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{address}"), fixture)
}

fn request() -> OneOffCheckoutRequest {
    OneOffCheckoutRequest::new(
        "tv0123456789abcdef",
        15_000,
        "BRL",
        "Aulas de novembro",
        "https://app.example/pagamento/retorno",
        "https://app.example/pagamento",
    )
    .unwrap()
    .with_customer_email("maria@example.com")
    .unwrap()
}

fn notification() -> PaymentNotification {
    PaymentNotification::new("tv0123456789abcdef")
        .unwrap()
        .with_transaction_id("nsu-1")
        .unwrap()
        .with_invoice_id("slug-1")
        .unwrap()
}

fn live(base: &str) -> InfinitePayProvider {
    InfinitePayProvider::checkout("$loja")
        .unwrap()
        .with_webhook_url("https://app.example/webhooks/infinitepay")
        .unwrap()
        .with_api_base(base)
}

#[tokio::test]
async fn links_and_payment_checks_follow_the_checkout_api() {
    let (base, fixture) = api().await;
    let provider = live(&base).with_api_key("inchk_fixture").unwrap();
    let checkout = provider.create_one_off_checkout(&request()).await.unwrap();
    assert_eq!(
        checkout.url(),
        "https://checkout.infinitepay.io/loja?lenc=abc"
    );
    assert!(!checkout.is_mock());
    assert_eq!(checkout.session_id(), None);
    let payment = provider
        .confirm_one_off_payment(&request(), &notification())
        .await
        .unwrap();
    assert_eq!(payment.state(), OneOffPaymentState::Paid);
    assert_eq!(payment.amount_minor(), 15_000);
    assert_eq!(payment.paid_amount_minor(), 15_600);
    assert_eq!(payment.transaction_id(), Some("nsu-1"));

    let seen = fixture.seen.lock().unwrap().clone();
    let (path, key, link) = &seen[0];
    assert_eq!(
        (path.as_str(), key.as_deref()),
        ("/links", Some("inchk_fixture"))
    );
    assert_eq!(
        link,
        &json!({
            "handle": "loja",
            "items": [{"quantity": 1, "price": 15_000, "description": "Aulas de novembro"}],
            "order_nsu": "tv0123456789abcdef",
            "redirect_url": "https://app.example/pagamento/retorno",
            "webhook_url": "https://app.example/webhooks/infinitepay",
            "customer": {"email": "maria@example.com"},
            "options": {"single_payment": true}
        })
    );
    let (path, _, check) = &seen[1];
    assert_eq!(path, "/payment_check");
    assert_eq!(
        check,
        &json!({"handle": "loja", "order_nsu": "tv0123456789abcdef", "transaction_nsu": "nsu-1", "slug": "slug-1"})
    );
}

#[tokio::test]
async fn without_an_api_key_the_link_has_no_options_or_header() {
    let (base, fixture) = api().await;
    let request = request().with_quantity(2).unwrap();
    live(&base).create_one_off_checkout(&request).await.unwrap();
    let (_, key, link) = fixture.seen.lock().unwrap()[0].clone();
    assert_eq!(key, None);
    assert!(link.get("options").is_none());
    assert_eq!(link["items"][0]["quantity"], 2);
    assert_eq!(link["items"][0]["price"], 15_000);
}

#[tokio::test]
async fn only_a_confirmed_payment_of_the_server_side_amount_counts() {
    let (base, fixture) = api().await;
    let provider = live(&base);
    for reply in [
        json!({"success": true, "paid": false, "amount": 15_000, "paid_amount": 0}),
        json!({"success": false, "paid": true, "amount": 15_000, "paid_amount": 15_000}),
        json!({}),
    ] {
        *fixture.check.lock().unwrap() = reply;
        let payment = provider
            .confirm_one_off_payment(&request(), &notification())
            .await
            .unwrap();
        assert_eq!(payment.state(), OneOffPaymentState::Unpaid);
    }
    // Anyone can create a link for a public handle with the same order_nsu:
    // a paid R$ 1.00 link never confirms the R$ 150.00 order.
    *fixture.check.lock().unwrap() =
        json!({"success": true, "paid": true, "amount": 100, "paid_amount": 100});
    assert!(matches!(
        provider
            .confirm_one_off_payment(&request(), &notification())
            .await,
        Err(CapitalError::InvalidCharge(_))
    ));
    *fixture.check.lock().unwrap() =
        json!({"success": true, "paid": true, "amount": 15_000, "paid_amount": 14_999});
    assert!(
        provider
            .confirm_one_off_payment(&request(), &notification())
            .await
            .is_err()
    );
    // A notification for another order, or without the identifiers
    // payment_check needs, never reaches the API.
    let calls = fixture.seen.lock().unwrap().len();
    for unusable in [
        PaymentNotification::new("another_order").unwrap(),
        PaymentNotification::new("tv0123456789abcdef").unwrap(),
        PaymentNotification::new("tv0123456789abcdef")
            .unwrap()
            .with_transaction_id("nsu-1")
            .unwrap(),
    ] {
        assert!(
            provider
                .confirm_one_off_payment(&request(), &unusable)
                .await
                .is_err()
        );
    }
    assert_eq!(fixture.seen.lock().unwrap().len(), calls);
}

#[tokio::test]
async fn link_responses_must_point_to_an_https_infinitepay_host() {
    let (base, fixture) = api().await;
    for reply in [
        json!({"url": "http://checkout.infinitepay.io/x"}),
        json!({"url": "https://infinitepay.io.example.com/x"}),
        json!({"url": "https://evilinfinitepay.io/x"}),
        json!({"link_url": "https://checkout.infinitepay.io/x"}),
    ] {
        *fixture.link.lock().unwrap() = reply;
        assert!(matches!(
            live(&base).create_one_off_checkout(&request()).await,
            Err(CapitalError::Provider(_))
        ));
    }
    *fixture.link.lock().unwrap() = json!({"checkout_url": "https://infinitepay.io/c/1"});
    assert_eq!(
        live(&base)
            .create_one_off_checkout(&request())
            .await
            .unwrap()
            .url(),
        "https://infinitepay.io/c/1"
    );
}

#[tokio::test]
async fn requests_outside_the_checkout_contract_fail_before_any_call() {
    let (base, fixture) = api().await;
    let provider = live(&base);
    let build = |amount: u64, currency: &str, success: &str| {
        OneOffCheckoutRequest::new(
            "order",
            amount,
            currency,
            "Item",
            success,
            "https://app.example/c",
        )
        .unwrap()
    };
    for request in [
        build(15_000, "usd", "https://app.example/ok"),
        build(99, "brl", "https://app.example/ok"),
    ] {
        assert!(matches!(
            provider.create_one_off_checkout(&request).await,
            Err(CapitalError::InvalidCharge(_))
        ));
    }
    assert!(matches!(
        provider
            .create_one_off_checkout(&build(100, "brl", "http://localhost:3000/ok"))
            .await,
        Err(CapitalError::ConfigurationError(_))
    ));
    let local_webhook = InfinitePayProvider::checkout("loja")
        .unwrap()
        .with_webhook_url("http://localhost:3000/webhooks")
        .unwrap()
        .with_api_base(&base);
    assert!(matches!(
        local_webhook.create_one_off_checkout(&request()).await,
        Err(CapitalError::ConfigurationError(_))
    ));
    assert!(fixture.seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn mock_handles_are_offline_and_never_paid() {
    for provider in [
        InfinitePayProvider::checkout("mock_loja").unwrap(),
        InfinitePayProvider::checkout("").unwrap(),
        InfinitePayProvider::new("mock_key", "mock_webhook"),
    ] {
        assert!(provider.is_mock());
        let checkout = provider.create_one_off_checkout(&request()).await.unwrap();
        assert!(checkout.is_mock());
        assert!(
            checkout
                .url()
                .starts_with("https://mock.infinitepay.invalid/")
        );
        let payment = provider
            .confirm_one_off_payment(&request(), &notification())
            .await
            .unwrap();
        assert_eq!(payment.state(), OneOffPaymentState::Mock);
        assert!(!payment.is_paid());
    }
}

#[test]
fn constructors_validate_handle_key_and_webhook_url() {
    assert!(InfinitePayProvider::checkout(" $loja-1.br_x ").is_ok());
    assert!(InfinitePayProvider::checkout("x".repeat(64)).is_ok());
    assert!(
        InfinitePayProvider::checkout("loja")
            .unwrap()
            .with_api_key(format!("inchk_{}", "x".repeat(194)))
            .is_ok(),
        "200 characters"
    );
    for handle in ["loja tia", "loja/1", "x".repeat(65).as_str()] {
        assert!(InfinitePayProvider::checkout(handle).is_err(), "{handle}");
    }
    let provider = || InfinitePayProvider::checkout("loja").unwrap();
    assert!(provider().with_api_key("inchk_ok").is_ok());
    for key in [
        "sk_live_x",
        "inchk_ x",
        format!("inchk_{}", "x".repeat(195)).as_str(),
    ] {
        assert!(provider().with_api_key(key).is_err(), "{key}");
    }
    assert!(
        provider()
            .with_webhook_url("https://app.example/hook")
            .is_ok()
    );
    for url in [
        "http://app.example/hook",
        "https://app.example/hook#x",
        "/hook",
    ] {
        assert!(provider().with_webhook_url(url).is_err(), "{url}");
    }
}

#[test]
fn notifications_read_identifiers_and_ignore_status_and_amounts() {
    let provider = InfinitePayProvider::checkout("loja").unwrap();
    assert_eq!(
        provider.webhook_verification_mode().unwrap(),
        WebhookVerificationMode::ConfirmedAtSource
    );
    let body = json!({
        "invoice_slug": "slug-1", "amount": 1, "paid_amount": 1, "installments": 1,
        "capture_method": "pix", "transaction_nsu": "nsu-1", "order_nsu": "tv0123456789abcdef",
        "receipt_url": "https://infinitepay.io/r", "items": []
    });
    let read = provider
        .parse_payment_notification(body.to_string().as_bytes(), &HashMap::new())
        .unwrap();
    assert_eq!(read, notification());
    let slug_only = json!({"order_nsu": "o1", "slug": "s1"});
    assert_eq!(
        provider
            .parse_payment_notification(slug_only.to_string().as_bytes(), &HashMap::new())
            .unwrap()
            .invoice_id(),
        Some("s1")
    );
    for invalid in [
        b"not json".to_vec(),
        json!({"transaction_nsu": "nsu-1"}).to_string().into_bytes(),
        json!({"order_nsu": "o 1"}).to_string().into_bytes(),
        json!({"order_nsu": "o1", "transaction_nsu": "a/b"})
            .to_string()
            .into_bytes(),
        {
            let mut body = json!({"order_nsu": "o1", "pad": ""})
                .to_string()
                .into_bytes();
            body.resize(64 * 1024 + 1, b' ');
            body
        },
    ] {
        assert!(matches!(
            provider.parse_payment_notification(&invalid, &HashMap::new()),
            Err(CapitalError::PayloadParseError(_))
        ));
    }
    let mut largest = json!({"order_nsu": "o1"}).to_string().into_bytes();
    largest.resize(64 * 1024, b' ');
    assert_eq!(
        provider
            .parse_payment_notification(&largest, &HashMap::new())
            .unwrap()
            .reference(),
        "o1"
    );
    // The unsigned body never becomes a WebhookEvent.
    assert!(matches!(
        provider.handle_webhook(body.to_string().as_bytes(), &HashMap::new()),
        Err(CapitalError::UnsupportedOperation(_))
    ));
}

#[path = "infinitepay_legacy_tests.rs"]
mod legacy;

/// InfinitePay's 404 body, with a `padding` field of `padding` bytes.
fn refusal(padding: usize) -> Value {
    json!({
        "success": false,
        "error": "external_checkout_not_enabled",
        "message": "External checkout is not enabled for this merchant. Enable it in your checkout settings to start creating payment links.",
        "redirect_url": "https://app.infinitepay.io/external-checkout#configuracoes?enabled=true",
        "padding": "x".repeat(padding),
    })
}

/// A `/links` endpoint that refuses with InfinitePay's 404 body, padded to
/// `padding` extra bytes.
async fn refusing_api(padding: usize) -> String {
    let body = refusal(padding);
    let app = Router::new().route(
        "/links",
        post(move || {
            let body = body.clone();
            async move { (axum::http::StatusCode::NOT_FOUND, Json(body)) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{address}")
}

#[tokio::test]
async fn a_refused_link_keeps_only_infinitepays_error_code() {
    let base = refusing_api(0).await;
    let Err(CapitalError::Provider(failure)) =
        live(&base).create_one_off_checkout(&request()).await
    else {
        panic!("a 404 is a provider failure");
    };
    assert_eq!(failure.status(), Some(404));
    assert_eq!(failure.reason(), Some("external_checkout_not_enabled"));
    let text = failure.to_string();
    assert_eq!(
        text,
        "infinitepay create payment link returned HTTP 404 (external_checkout_not_enabled)"
    );
    assert!(
        !text.contains("Enable it"),
        "the provider's message is not kept"
    );

    // A body of exactly 4 KiB is read; one byte more is not.
    let exact = 4096 - serde_json::to_vec(&refusal(0)).unwrap().len();
    let base = refusing_api(exact).await;
    let Err(CapitalError::Provider(failure)) = live(&base).create_one_off_checkout(&request()).await
    else {
        panic!("a 404 is a provider failure");
    };
    assert_eq!(failure.reason(), Some("external_checkout_not_enabled"));
    let base = refusing_api(exact + 1).await;
    let Err(CapitalError::Provider(failure)) = live(&base).create_one_off_checkout(&request()).await
    else {
        panic!("a 404 is a provider failure");
    };
    assert_eq!(failure.reason(), None);
    // A much larger body is not read either.
    let base = refusing_api(5_000).await;
    let Err(CapitalError::Provider(failure)) =
        live(&base).create_one_off_checkout(&request()).await
    else {
        panic!("a 404 is a provider failure");
    };
    assert_eq!(failure.status(), Some(404));
    assert_eq!(failure.reason(), None);
}
