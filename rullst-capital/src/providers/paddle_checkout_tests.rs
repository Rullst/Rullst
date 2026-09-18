use super::*;
use crate::providers::paddle_test_support::*;

fn provisioning() -> PaddleCustomerRequest {
    PaddleCustomerRequest::new("owner_opaque", "provision_opaque", "contact@example.test").unwrap()
}

#[tokio::test]
async fn wire_customer_checkout_recovery_and_environment_contract() {
    let fixture = fixture().await;
    let customer = fixture
        .provider
        .create_customer(&provisioning())
        .await
        .unwrap();
    assert_eq!(customer.id(), CUSTOMER);
    assert_eq!(customer.sandbox(), Some(true));
    assert!(!customer.is_mock());
    assert_eq!(customer.request_digest(), provisioning().request_digest());
    let created = fixture
        .provider
        .create_transaction_checkout(&request())
        .await
        .unwrap();
    assert_eq!(created.id(), TRANSACTION);
    assert_eq!(created.status(), "draft");
    assert_eq!(
        created.url(),
        Some(format!("https://app.example/pay?_ptxn={TRANSACTION}").as_str())
    );
    assert_eq!(created.sandbox(), Some(true));
    assert!(!created.is_mock());
    assert!(created.subscription_id().is_none());
    assert_eq!(created.request_digest(), request().request_digest());
    let mut changed = customer_body();
    changed["email"] = json!("updated@example.test");
    fixture.respond("GET", &format!("/customers/{CUSTOMER}"), 200, changed);
    assert_eq!(
        fixture
            .provider
            .retrieve_bound_customer(&provisioning(), CUSTOMER)
            .await
            .unwrap()
            .id(),
        CUSTOMER
    );
    let recovered = fixture
        .provider
        .retrieve_transaction_checkout(&request(), TRANSACTION)
        .await
        .unwrap();
    assert_eq!(recovered.request_digest(), created.request_digest());
    let calls = fixture.requests.lock().unwrap();
    assert_eq!(
        calls
            .iter()
            .map(|(method, path, _)| format!("{method} {path}"))
            .collect::<Vec<_>>(),
        vec![
            "POST /customers".to_string(),
            format!("GET /customers/{CUSTOMER}"),
            "POST /transactions".into(),
            format!("GET /customers/{CUSTOMER}"),
            format!("GET /transactions/{TRANSACTION}")
        ]
    );
    assert_eq!(
        calls[0].2,
        json!({"email":"contact@example.test", "custom_data":{"rullst_owner_reference":"owner_opaque", "rullst_attempt_reference":"provision_opaque"}})
    );
    assert_eq!(calls[2].2, checkout_body(&request()));
    assert!(calls[2].2.get("customer").is_none());
    assert!(calls[2].2.get("return_url").is_none());
    for sandbox in [false, true] {
        let provider = PaddleProvider::new("unused", "mock_webhook").with_sandbox(sandbox);
        let wire = provider
            .billing_request(
                Method::POST,
                "/transactions",
                Some(checkout_body(&request())),
                "checkout",
            )
            .unwrap();
        assert_eq!(
            wire.url().host_str(),
            Some(if sandbox {
                "sandbox-api.paddle.com"
            } else {
                "api.paddle.com"
            })
        );
        assert_eq!(wire.headers()["paddle-version"], "1");
    }
}
fn customer_body() -> Value {
    crate::providers::paddle_test_support::customer()
}

#[tokio::test]
async fn foreign_customer_stops_before_mutation_and_provider_errors_are_not_retried() {
    let fixture = fixture().await;
    let mut bad = customer_body();
    bad["custom_data"]["rullst_owner_reference"] = json!("other_owner");
    fixture.respond("GET", &format!("/customers/{CUSTOMER}"), 200, bad);
    assert!(
        fixture
            .provider
            .create_transaction_checkout(&request())
            .await
            .is_err()
    );
    assert_eq!(fixture.requests.lock().unwrap().len(), 1);
    fixture.respond(
        "GET",
        &format!("/customers/{CUSTOMER}"),
        200,
        customer_body(),
    );
    fixture.respond(
        "POST",
        "/transactions",
        503,
        json!({"secret":"must not leak"}),
    );
    let error = fixture
        .provider
        .create_transaction_checkout(&request())
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("must not leak"));
    assert_eq!(
        fixture
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(method, _, _)| method == "POST")
            .count(),
        1
    );
    assert!(
        fixture
            .provider
            .retrieve_transaction_checkout(&request(), "txn_../foreign")
            .await
            .is_err()
    );
    assert!(
        fixture
            .provider
            .retrieve_bound_customer(&provisioning(), "ctm_/foreign")
            .await
            .is_err()
    );
    assert_eq!(fixture.requests.lock().unwrap().len(), 3);
}

#[test]
fn customer_and_transaction_parsers_reject_confused_bindings_and_payment_links() {
    let good = customer_body();
    for (pointer, value) in [
        ("/id", json!("ctm_bad")),
        ("/status", json!("archived")),
        ("/email", json!("other@example.test")),
        ("/custom_data/rullst_owner_reference", json!("foreign")),
        ("/custom_data/rullst_attempt_reference", json!("foreign")),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            parse_customer(&provisioning(), &bad, None, true, false).is_err(),
            "{pointer}"
        );
    }
    assert!(parse_customer(&provisioning(), &good, Some("ctm_other"), false, true).is_err());
    let good = transaction();
    for (pointer, value) in [
        ("/id", json!("txn_bad")),
        ("/status", json!("completed")),
        ("/origin", json!("subscription_recurring")),
        ("/collection_mode", json!("manual")),
        ("/customer_id", json!("ctm_other")),
        ("/custom_data/rullst_owner_reference", json!("foreign")),
        ("/custom_data/rullst_attempt_reference", json!("foreign")),
        ("/items/0/price/id", json!("pri_other")),
        ("/items/0/quantity", json!(2)),
        ("/items/0/price/billing_cycle", Value::Null),
        ("/items/0/price/billing_cycle/frequency", json!(0)),
        ("/items/0/price/billing_cycle/interval", json!("invalid")),
        ("/checkout/url", Value::Null),
        ("/subscription_id", json!(SUBSCRIPTION)),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            parse_transaction(&request(), &bad, None, true, true).is_err(),
            "{pointer}"
        );
    }
    for url in [
        format!("https://foreign.example/pay?_ptxn={TRANSACTION}"),
        format!("https://app.example/other?_ptxn={TRANSACTION}"),
        "https://app.example/pay?_ptxn=txn_other".into(),
        format!("https://app.example/pay?_ptxn={TRANSACTION}&_ptxn={TRANSACTION}"),
        format!("https://app.example/pay?_ptxn={TRANSACTION}&foreign=1"),
        format!("http://app.example/pay?_ptxn={TRANSACTION}"),
        format!("https://user:secret@app.example/pay?_ptxn={TRANSACTION}"),
        format!("https://app.example/pay?_ptxn={TRANSACTION}#x"),
    ] {
        let mut bad = good.clone();
        bad["checkout"]["url"] = json!(url);
        assert!(parse_transaction(&request(), &bad, None, true, true).is_err());
    }
    assert!(parse_transaction(&request(), &good, Some("txn_other"), true, false).is_err());
    for status in [
        "ready",
        "billed",
        "paid",
        "completed",
        "canceled",
        "past_due",
    ] {
        let mut value = good.clone();
        value["status"] = json!(status);
        if !matches!(status, "ready" | "billed") {
            value["checkout"] = Value::Null;
            value["subscription_id"] = json!(SUBSCRIPTION);
        }
        let parsed =
            parse_transaction(&request(), &value, Some(TRANSACTION), false, false).unwrap();
        assert_eq!(parsed.status(), status);
    }
    let mut bad = good.clone();
    bad["subscription_id"] = json!(42);
    assert!(parse_transaction(&request(), &bad, None, true, false).is_err());
    bad = good;
    bad["status"] = json!("unknown");
    assert!(parse_transaction(&request(), &bad, None, true, false).is_err());
}

#[tokio::test]
async fn mock_results_are_deterministic_distinct_and_debug_redacts_inputs() {
    let provider = PaddleProvider::new("mock_key", "mock_secret");
    let input = provisioning();
    assert_eq!(input.owner_reference(), "owner_opaque");
    assert_eq!(input.attempt_reference(), "provision_opaque");
    let customer = provider.create_customer(&input).await.unwrap();
    assert!(customer.is_mock());
    assert_eq!(
        provider
            .retrieve_bound_customer(&input, customer.id())
            .await
            .unwrap()
            .id(),
        customer.id()
    );
    assert!(
        provider
            .retrieve_bound_customer(&input, CUSTOMER)
            .await
            .is_err()
    );
    let input = request();
    assert_eq!(input.customer_id(), CUSTOMER);
    assert_eq!(input.price_id(), PRICE);
    assert_eq!(input.owner_reference(), "owner_opaque");
    assert_eq!(input.attempt_reference(), "attempt_opaque");
    assert_eq!(input.payment_link(), "https://app.example/pay");
    let first = provider.create_transaction_checkout(&input).await.unwrap();
    assert!(first.is_mock());
    assert_eq!(
        provider
            .create_transaction_checkout(&input)
            .await
            .unwrap()
            .id(),
        first.id()
    );
    assert_eq!(
        provider
            .retrieve_transaction_checkout(&input, first.id())
            .await
            .unwrap()
            .id(),
        first.id()
    );
    assert!(
        provider
            .retrieve_transaction_checkout(&input, TRANSACTION)
            .await
            .is_err()
    );
    let debug = format!("{input:?}{first:?}{customer:?}{:?}", provisioning());
    for private in [
        CUSTOMER,
        PRICE,
        "owner_opaque",
        "contact@example.test",
        "app.example",
    ] {
        assert!(!debug.contains(private));
    }
    for url in [
        "http://app.example/pay",
        "https://app.example/pay?x=1",
        "https://app.example/pay#x",
    ] {
        assert!(PaddleCheckoutRequest::new(CUSTOMER, PRICE, "owner", "attempt", url).is_err());
    }
    for (customer, price, owner, attempt) in [
        ("ctm_bad", PRICE, "owner", "attempt"),
        (CUSTOMER, "pri_bad", "owner", "attempt"),
        (CUSTOMER, PRICE, "email@example.test", "attempt"),
        (CUSTOMER, PRICE, "owner", ""),
    ] {
        assert!(
            PaddleCheckoutRequest::new(customer, price, owner, attempt, "https://app.example/pay")
                .is_err()
        );
    }
    assert!(PaddleCustomerRequest::new("owner", "attempt", "invalid").is_err());
    assert!(PaddleCustomerRequest::new("", "attempt", "contact@example.test").is_err());
    assert_ne!(
        request().request_digest(),
        PaddleCheckoutRequest::new(
            CUSTOMER,
            PRICE,
            "owner_opaque",
            "other",
            "https://app.example/pay"
        )
        .unwrap()
        .request_digest()
    );
}
