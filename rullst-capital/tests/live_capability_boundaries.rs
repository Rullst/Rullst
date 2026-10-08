//! Live credentials must not turn unimplemented billing operations into success.
use rullst_capital::{CapitalError, providers::*};

#[test]
fn authenticated_incomplete_events_cannot_grant_active_access() {
    use ring::hmac;
    use std::collections::HashMap;
    let payload = b"{}";
    let secret = "secret";
    let now = chrono::Utc::now().timestamp();
    let sign = |bytes: &[u8]| {
        hex::encode(
            hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes()), bytes).as_ref(),
        )
    };
    let mut accepted = Vec::new();
    for provider in providers("fixture-key") {
        let (name, signature) = match provider.name() {
            "stripe" => (
                "stripe-signature",
                format!("t={now},v1={}", sign(format!("{now}.{{}}").as_bytes())),
            ),
            "infinitepay" => ("x-infinitepay-signature", sign(payload)),
            _ => continue,
        };
        let headers = HashMap::from([(name.to_owned(), signature)]);
        if provider.handle_webhook(payload, &headers).is_ok() {
            accepted.push(provider.name());
        }
    }
    assert!(
        accepted.is_empty(),
        "authenticated payload without status accepted: {accepted:?}"
    );
}

#[tokio::test]
async fn plan_only_checkout_rejects_adapters_without_authoritative_pricing() {
    // A newline makes header construction fail locally in the old implementation,
    // so even the red regression never sends credentials to a real provider.
    let mut attempted_live = Vec::new();
    for provider in providers("fixture\ninvalid-header") {
        if provider.name() != "infinitepay" {
            continue;
        }
        let result = provider
            .create_checkout_session(
                "user@example.invalid",
                "plan_without_a_price",
                "https://app.example/billing",
            )
            .await;
        if !matches!(result, Err(CapitalError::UnsupportedOperation(_))) {
            attempted_live.push(provider.name());
        }
    }
    assert!(
        attempted_live.is_empty(),
        "unpriced live checkout attempted: {attempted_live:?}"
    );
}

#[tokio::test]
async fn placeholder_credentials_do_not_silently_enable_mock_checkout() {
    let providers: Vec<Box<dyn BillingProvider>> = vec![Box::new(InfinitePayProvider::new(
        "handle_fixture",
        "secret",
    ))];
    let mut fabricated = Vec::new();
    for provider in providers {
        if !matches!(
            provider
                .create_checkout_session("user@example.invalid", "plan", "https://app.example")
                .await,
            Err(CapitalError::UnsupportedOperation(_))
        ) {
            fabricated.push(provider.name());
        }
    }
    assert!(
        fabricated.is_empty(),
        "undocumented mock credential aliases: {fabricated:?}"
    );
}

fn providers(key: &str) -> Vec<Box<dyn BillingProvider>> {
    vec![
        Box::new(StripeProvider::new(key, "secret")),
        Box::new(InfinitePayProvider::new(key, "secret")),
    ]
}

#[tokio::test]
async fn live_portal_creation_requires_a_reviewed_provider_session_contract() {
    let mut incorrectly_successful = Vec::new();
    for provider in providers("live-fixture-key") {
        if !matches!(
            provider
                .create_customer_portal("user@example.invalid", "https://app.example/billing")
                .await,
            Err(CapitalError::UnsupportedOperation(_))
        ) {
            incorrectly_successful.push(provider.name());
        }
    }
    assert!(
        incorrectly_successful.is_empty(),
        "unimplemented live portals: {incorrectly_successful:?}"
    );
    for key in ["", "mock_key"] {
        for provider in providers(key) {
            assert!(
                provider
                    .create_customer_portal("user@example.invalid", "https://app.example/billing")
                    .await
                    .is_ok()
            );
        }
    }
}

#[tokio::test]
async fn live_subscription_noops_fail_closed() {
    let mut incorrectly_successful = Vec::new();
    for provider in providers("live-fixture-key") {
        let result = match provider.name() {
            "infinitepay" => provider.cancel_subscription("sub_1").await,
            _ => continue,
        };
        if !matches!(result, Err(CapitalError::UnsupportedOperation(_))) {
            incorrectly_successful.push(provider.name());
        }
    }
    assert!(
        incorrectly_successful.is_empty(),
        "unimplemented live operations: {incorrectly_successful:?}"
    );
}
