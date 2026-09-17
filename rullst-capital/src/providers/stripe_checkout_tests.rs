use super::*;
use serde_json::json;

fn request() -> StripeCheckoutRequest {
    StripeCheckoutRequest::new(
        "cus_fixture",
        "price_fixture",
        "owner_fixture",
        "attempt_fixture",
        "https://app.example/success?session={CHECKOUT_SESSION_ID}&flow=billing",
        "https://app.example/cancel",
    )
    .unwrap()
}

fn response() -> Value {
    let request = request();
    json!({
        "id":"cs_test_fixture", "object":"checkout.session", "mode":"subscription", "status":"open",
        "customer":request.customer_id(), "client_reference_id":request.owner_reference(),
        "success_url":request.success_url(), "cancel_url":request.cancel_url(),
        "livemode":false, "expires_at":1800000000,
        "url":"https://checkout.stripe.com/c/pay/cs_test_fixture#opaque-client-state",
        "line_items":{"has_more":false,"data":[{"quantity":1,"price":{"id":"price_fixture","type":"recurring"}}]}
    })
}

#[test]
fn outbound_request_binds_customer_owner_price_and_retry_without_email() {
    let request = request();
    let outbound = build_request(&reqwest::Client::new(), "sk_test_fixture", &request).unwrap();
    assert_eq!(
        outbound.url().as_str(),
        "https://api.stripe.com/v1/checkout/sessions"
    );
    assert_eq!(
        outbound.headers()["Idempotency-Key"],
        request.idempotency_key()
    );
    assert_eq!(outbound.headers()["Stripe-Version"], "2025-03-31.basil");
    let body = std::str::from_utf8(outbound.body().unwrap().as_bytes().unwrap()).unwrap();
    // Parse as a form to detect unescaped redirect query parameters, not just
    // whether individual strings appeared in the constructed body.
    let form = reqwest::Url::parse(&format!("https://fixture.invalid/?{body}")).unwrap();
    let fields = form
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(fields.len(), 9);
    for (name, value) in [
        ("mode", "subscription"),
        ("customer", request.customer_id()),
        ("client_reference_id", request.owner_reference()),
        (
            "subscription_data[metadata][rullst_owner_reference]",
            request.owner_reference(),
        ),
        ("line_items[0][price]", request.price_id()),
        ("line_items[0][quantity]", "1"),
        ("success_url", request.success_url()),
        ("cancel_url", request.cancel_url()),
        ("expand[0]", "line_items"),
    ] {
        assert_eq!(fields.get(name).map(|value| value.as_ref()), Some(value));
    }
    assert!(!fields.contains_key("customer_email"));
}

#[test]
fn open_session_is_bound_to_customer_reference_mode_redirects_and_line_items() {
    let request = request();
    let valid = response();
    let session = parse_response(&request, &valid).unwrap();
    assert_eq!(session.status(), StripeCheckoutStatus::Created);
    assert_eq!(session.request_digest(), request.request_digest());
    assert_eq!(session.livemode(), Some(false));
    assert_eq!(session.expires_at(), Some(1800000000));
    assert!(session.url().ends_with("#opaque-client-state"));
    for (field, invalid) in [
        ("customer", json!("cus_other")),
        ("client_reference_id", json!("owner_other")),
        ("id", json!("pi_payment")),
        ("object", json!("payment_intent")),
        ("mode", json!("payment")),
        ("status", json!("complete")),
        ("success_url", json!("https://other.example/")),
        ("cancel_url", json!("https://other.example/")),
        ("livemode", json!("false")),
        ("expires_at", json!(-1)),
        ("url", json!("https://secret@checkout.stripe.com/session")),
    ] {
        let mut bad = valid.clone();
        bad[field] = invalid;
        assert!(
            parse_response(&request, &bad).is_err(),
            "accepted invalid {field}"
        );
        bad[field] = Value::Null;
        assert!(
            parse_response(&request, &bad).is_err(),
            "accepted missing {field}"
        );
    }
    for invalid in [
        json!({"has_more":false,"data":[]}),
        json!({"has_more":true,"data":[{"quantity":1,"price":{"id":"price_fixture","type":"recurring"}}]}),
        json!({"has_more":false,"data":[{"quantity":2,"price":{"id":"price_fixture","type":"recurring"}}]}),
        json!({"has_more":false,"data":[{"quantity":1,"price":{"id":"price_other","type":"recurring"}}]}),
        json!({"has_more":false,"data":[{"quantity":1,"price":{"id":"price_fixture","type":"one_time"}}]}),
    ] {
        let mut bad = valid.clone();
        bad["line_items"] = invalid;
        assert!(parse_response(&request, &bad).is_err());
    }
}

#[test]
fn changed_input_changes_persisted_digest_and_debug_omits_private_fields() {
    let fields = [
        "cus_fixture",
        "price_fixture",
        "owner_fixture",
        "attempt_fixture",
        "https://app.example/success",
        "https://app.example/cancel",
    ];
    let make = |fields: [&str; 6]| {
        StripeCheckoutRequest::new(
            fields[0], fields[1], fields[2], fields[3], fields[4], fields[5],
        )
        .unwrap()
    };
    let original = make(fields);
    for (index, value) in [
        "cus_other",
        "price_other",
        "owner_other",
        "attempt_other",
        "https://other.example/success",
        "https://other.example/cancel",
    ]
    .into_iter()
    .enumerate()
    {
        let mut changed = fields;
        changed[index] = value;
        assert_ne!(original.request_digest(), make(changed).request_digest());
    }
    let debug = format!(
        "{original:?} {:?}",
        parse_response(&request(), &response()).unwrap()
    );
    for secret in fields
        .into_iter()
        .chain(["cs_test_fixture", "opaque-client-state"])
    {
        assert!(!debug.contains(secret));
    }
}

#[tokio::test]
async fn mocks_are_repeatable_and_cannot_be_mistaken_for_created_sessions() {
    for key in ["", "mock_fixture"] {
        let provider = super::super::StripeProvider::new(key, "mock_webhook");
        let first = provider
            .create_subscription_checkout(&request())
            .await
            .unwrap();
        let replay = provider
            .create_subscription_checkout(&request())
            .await
            .unwrap();
        assert_eq!(first.id(), replay.id());
        assert_eq!(first.url(), replay.url());
        assert_eq!(first.status(), StripeCheckoutStatus::Mock);
        assert_eq!(first.livemode(), None);
        assert_eq!(first.expires_at(), None);
    }
}

#[test]
fn invalid_identity_keys_and_redirects_fail_before_dispatch() {
    let fields = [
        "cus_fixture",
        "price_fixture",
        "owner_fixture",
        "attempt_fixture",
        "https://app.example/success",
        "https://app.example/cancel",
    ];
    for (index, invalid) in [
        (0, "cus_"),
        (0, "user@example.com"),
        (1, "plan_fixture"),
        (2, "user@example.com"),
        (3, "bad\nheader"),
        (3, ""),
        (4, "http://app.example/success"),
        (5, "https://secret@app.example/cancel"),
        (4, "https://app.example/success#fragment"),
        (5, "\nhttps://app.example/cancel"),
    ] {
        let mut bad = fields;
        bad[index] = invalid;
        assert!(
            StripeCheckoutRequest::new(bad[0], bad[1], bad[2], bad[3], bad[4], bad[5]).is_err()
        );
    }
    assert!(
        StripeCheckoutRequest::new(
            fields[0],
            fields[1],
            fields[2],
            "x".repeat(256),
            fields[4],
            fields[5]
        )
        .is_err()
    );
}
