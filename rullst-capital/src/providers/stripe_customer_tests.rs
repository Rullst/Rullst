use super::*;
use serde_json::json;

fn request() -> StripeCustomerRequest {
    StripeCustomerRequest::new("owner_fixture", "provision_fixture").unwrap()
}

fn response() -> Value {
    json!({"id":"cus_fixture", "object":"customer", "livemode":false,
        "created":1750000000, "metadata":{"rullst_owner_reference":"owner_fixture"}})
}

#[test]
fn provisioning_forwards_only_explicit_contact_metadata_and_retry_fields() {
    for with_email in [false, true] {
        let request = if with_email {
            request()
                .with_email("billing+check&alias@example.com")
                .unwrap()
        } else {
            request()
        };
        let outbound = build_request(&reqwest::Client::new(), "sk_test_fixture", &request).unwrap();
        assert_eq!(outbound.method(), reqwest::Method::POST);
        assert_eq!(
            outbound.url().as_str(),
            "https://api.stripe.com/v1/customers"
        );
        assert_eq!(outbound.headers()["Stripe-Version"], "2025-03-31.basil");
        assert_eq!(
            outbound.headers()["Idempotency-Key"],
            request.idempotency_key()
        );
        let body = std::str::from_utf8(outbound.body().unwrap().as_bytes().unwrap()).unwrap();
        let form = reqwest::Url::parse(&format!("https://fixture.invalid/?{body}")).unwrap();
        let fields = form
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(fields.len(), if with_email { 2 } else { 1 });
        assert_eq!(fields["metadata[rullst_owner_reference]"], "owner_fixture");
        assert_eq!(
            fields.get("email").map(|value| value.as_ref()),
            request.email()
        );
    }
}

#[test]
fn returned_customer_is_bound_to_metadata_identity_and_credential_mode() {
    let request = request();
    let body = response();
    let receipt = parse_response(&request, &body, Some(false)).unwrap();
    assert_eq!(receipt.id(), "cus_fixture");
    assert_eq!(receipt.status(), StripeCustomerStatus::Created);
    assert_eq!(receipt.created_at(), Some(1750000000));
    assert_eq!(receipt.livemode(), Some(false));
    assert_eq!(receipt.request_digest(), request.request_digest());
    assert!(parse_response(&request, &body, Some(true)).is_err());
    for (field, invalid) in [
        ("id", json!("sub_confused")),
        ("object", json!("deleted_customer")),
        ("created", json!(0)),
        ("livemode", json!("false")),
        ("metadata", json!({"rullst_owner_reference":"owner_other"})),
    ] {
        let mut bad = body.clone();
        bad[field] = invalid;
        assert!(
            parse_response(&request, &bad, Some(false)).is_err(),
            "accepted invalid {field}"
        );
        bad[field] = Value::Null;
        assert!(
            parse_response(&request, &bad, Some(false)).is_err(),
            "accepted missing {field}"
        );
    }
    for deleted in [json!(true), json!("false")] {
        let mut bad = body.clone();
        bad["deleted"] = deleted;
        assert!(parse_response(&request, &bad, Some(false)).is_err());
    }
    for key in ["sk_test_fixture", "rk_test_fixture"] {
        assert_eq!(stripe_contract::credential_mode(key), Some(false));
    }
    for key in ["sk_live_fixture", "rk_live_fixture"] {
        assert_eq!(stripe_contract::credential_mode(key), Some(true));
    }
    assert_eq!(stripe_contract::credential_mode("opaque_credential"), None);
}

#[tokio::test]
async fn deterministic_customer_mocks_feed_explicit_checkout_mocks() {
    for key in ["", "mock_api"] {
        let provider = super::super::StripeProvider::new(key, "mock_webhook");
        let customer = provider.create_customer(&request()).await.unwrap();
        let replay = provider.create_customer(&request()).await.unwrap();
        assert_eq!(customer.id(), replay.id());
        assert_eq!(customer.status(), StripeCustomerStatus::Mock);
        assert_eq!(customer.livemode(), None);
        assert_eq!(customer.created_at(), None);
        let checkout = crate::StripeCheckoutRequest::new(
            customer.id(),
            "price_fixture",
            request().owner_reference(),
            "checkout_fixture",
            "https://app.example/success",
            "https://app.example/cancel",
        )
        .unwrap();
        let session = provider
            .create_subscription_checkout(&checkout)
            .await
            .unwrap();
        assert_eq!(session.status(), crate::StripeCheckoutStatus::Mock);
    }
}

#[test]
fn changed_provisioning_input_changes_digest_and_debug_redacts_contact_and_ids() {
    let original = request();
    let with_email = original.clone().with_email("private@example.com").unwrap();
    for changed in [
        StripeCustomerRequest::new("owner_other", original.idempotency_key()).unwrap(),
        StripeCustomerRequest::new(original.owner_reference(), "provision_other").unwrap(),
        with_email.clone(),
    ] {
        assert_ne!(original.request_digest(), changed.request_digest());
    }
    assert_ne!(
        with_email.request_digest(),
        original
            .clone()
            .with_email("other@example.com")
            .unwrap()
            .request_digest()
    );
    let debug = format!(
        "{with_email:?} {:?}",
        parse_response(&original, &response(), Some(false)).unwrap()
    );
    for private in [
        "private@example.com",
        "owner_fixture",
        "provision_fixture",
        "cus_fixture",
    ] {
        assert!(!debug.contains(private));
    }
}

#[test]
fn invalid_provisioning_inputs_fail_before_network_dispatch() {
    for invalid in [
        "",
        "owner email@example.com",
        "key\r\nInjected",
        "invalid/path",
    ] {
        assert!(StripeCustomerRequest::new(invalid, "valid").is_err());
        assert!(StripeCustomerRequest::new("valid", invalid).is_err());
    }
    assert!(StripeCustomerRequest::new("x".repeat(201), "valid").is_err());
    assert!(StripeCustomerRequest::new("valid", "x".repeat(256)).is_err());
    for invalid in [
        "",
        "missing-at",
        "@example.com",
        "user@",
        "user@a@b",
        "user @example.com",
        "u\n@example.com",
    ] {
        assert!(request().with_email(invalid).is_err());
    }
    assert!(
        request()
            .with_email(format!("{}@example.com", "x".repeat(254)))
            .is_err()
    );
}
