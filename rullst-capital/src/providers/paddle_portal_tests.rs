use super::*;
use crate::ProviderFailureKind;
use crate::providers::paddle_test_support::{CUSTOMER, customer, fixture};
use serde_json::json;

const SESSION: &str = "cpls_01hv6y1jedq4p1n0yqn5ba3ky4";
const PORTAL: &str = "cpl_01hv6y1jedq4p1n0yqn5ba3ky4";
const TOKEN: &str = "fixture.bearer.credential";

fn provisioning() -> PaddleCustomerRequest {
    PaddleCustomerRequest::new("owner_opaque", "provision_opaque", "contact@example.test").unwrap()
}
fn portal_path() -> String {
    format!("/customers/{CUSTOMER}/portal-sessions")
}
fn overview(sandbox: bool) -> String {
    let prefix = if sandbox { "sandbox-" } else { "" };
    format!("https://{prefix}customer-portal.paddle.com/{PORTAL}?action=overview&token={TOKEN}")
}
fn session(sandbox: bool) -> Value {
    json!({"id":SESSION,"customer_id":CUSTOMER,"urls":{
        "general":{"overview":overview(sandbox)},"subscriptions":[]}})
}
fn assert_mismatch(error: CapitalError) {
    assert!(!format!("{error:?} {error}").contains(TOKEN));
    let CapitalError::Provider(failure) = error else {
        panic!("expected typed provider failure");
    };
    assert_eq!(failure.kind(), ProviderFailureKind::ContractMismatch);
}

#[tokio::test]
async fn authenticates_bound_customer_before_creating_fresh_overview_sessions() {
    for sandbox in [true, false] {
        let mut f = fixture().await;
        f.provider.sandbox = sandbox;
        // Contact changes must not transfer ownership or prevent valid access.
        let mut current = customer();
        current["email"] = json!("changed@example.test");
        f.respond("GET", &format!("/customers/{CUSTOMER}"), 200, current);
        f.respond("POST", &portal_path(), 201, session(sandbox));
        for _ in 0..2 {
            let receipt = f
                .provider
                .create_bound_customer_portal(&provisioning(), CUSTOMER)
                .await
                .unwrap();
            assert_eq!(receipt.id(), SESSION);
            assert_eq!(receipt.customer_id(), CUSTOMER);
            assert_eq!(receipt.url(), overview(sandbox));
            assert_eq!(receipt.sandbox(), Some(sandbox));
            assert!(!receipt.is_mock());
            receipt.require_real().unwrap();
            let debug = format!("{receipt:?}");
            for secret in [TOKEN, SESSION, CUSTOMER, "https://"] {
                assert!(!debug.contains(secret));
            }
        }
        assert_eq!(
            *f.requests.lock().unwrap(),
            vec![
                ("GET".into(), format!("/customers/{CUSTOMER}"), Value::Null),
                ("POST".into(), portal_path(), Value::Null),
                ("GET".into(), format!("/customers/{CUSTOMER}"), Value::Null),
                ("POST".into(), portal_path(), Value::Null),
            ]
        );
    }
}

#[tokio::test]
async fn rejects_foreign_archived_or_unbound_customers_before_portal_creation() {
    for (pointer, value) in [
        ("/id", json!("ctm_01hv6y1jedq4p1n0yqn5ba3ky5")),
        ("/status", json!("archived")),
        (
            "/custom_data/rullst_owner_reference",
            json!("another_owner"),
        ),
        (
            "/custom_data/rullst_attempt_reference",
            json!("another_attempt"),
        ),
        ("/custom_data", Value::Null),
    ] {
        let f = fixture().await;
        let mut current = customer();
        *current.pointer_mut(pointer).unwrap() = value;
        f.respond("GET", &format!("/customers/{CUSTOMER}"), 200, current);
        f.respond("POST", &portal_path(), 201, session(true));
        assert_mismatch(
            f.provider
                .create_bound_customer_portal(&provisioning(), CUSTOMER)
                .await
                .unwrap_err(),
        );
        let requests = f.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].0, "GET");
    }
}

#[tokio::test]
async fn rechecks_owner_after_an_earlier_successful_session() {
    let f = fixture().await;
    f.respond("POST", &portal_path(), 201, session(true));
    f.provider
        .create_bound_customer_portal(&provisioning(), CUSTOMER)
        .await
        .unwrap();
    let mut current = customer();
    current["custom_data"]["rullst_owner_reference"] = json!("another_owner");
    f.respond("GET", &format!("/customers/{CUSTOMER}"), 200, current);
    assert_mismatch(
        f.provider
            .create_bound_customer_portal(&provisioning(), CUSTOMER)
            .await
            .unwrap_err(),
    );
    assert_eq!(f.requests.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn invalid_customer_ids_never_reach_transport() {
    let f = fixture().await;
    for customer in [
        "",
        "ctm_short",
        "../customers",
        "contact@example.test",
        SESSION,
    ] {
        assert!(matches!(
            f.provider
                .create_bound_customer_portal(&provisioning(), customer)
                .await,
            Err(CapitalError::ConfigurationError(_))
        ));
    }
    assert!(f.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn failures_are_redacted_and_neither_reads_nor_creations_retry() {
    for method in ["GET", "POST"] {
        for status in [302, 401, 403, 429, 500, 503] {
            let f = fixture().await;
            let path = if method == "GET" {
                format!("/customers/{CUSTOMER}")
            } else {
                portal_path()
            };
            f.respond(method, &path, status, json!({"secret":TOKEN}));
            let error = f
                .provider
                .create_bound_customer_portal(&provisioning(), CUSTOMER)
                .await
                .unwrap_err();
            assert!(!format!("{error:?} {error}").contains(TOKEN));
            let CapitalError::Provider(failure) = error else {
                panic!("expected provider failure")
            };
            assert_eq!(failure.kind(), ProviderFailureKind::HttpResponse);
            assert_eq!(failure.status(), Some(status));
            assert_eq!(
                f.requests.lock().unwrap().len(),
                if method == "GET" { 1 } else { 2 }
            );
        }
    }
}

#[tokio::test]
async fn refuses_inconsistent_successful_portal_responses() {
    for (pointer, value) in [
        ("/id", json!("cpls_short")),
        ("/id", Value::Null),
        ("/customer_id", json!("ctm_01hv6y1jedq4p1n0yqn5ba3ky5")),
        ("/customer_id", Value::Null),
        ("/urls/general/overview", json!(overview(false))),
        ("/urls/general/overview", Value::Null),
        ("/urls", Value::Null),
    ] {
        let f = fixture().await;
        let mut response = session(true);
        *response.pointer_mut(pointer).unwrap() = value;
        f.respond("POST", &portal_path(), 201, response);
        assert_mismatch(
            f.provider
                .create_bound_customer_portal(&provisioning(), CUSTOMER)
                .await
                .unwrap_err(),
        );
        assert_eq!(f.requests.lock().unwrap().len(), 2);
    }
}

#[test]
fn portal_destination_rejects_environment_confusion_ambiguity_and_bearer_leaks() {
    let valid = overview(true);
    for url in [
        "".into(),
        "not a URL".into(),
        valid.replace("https:", "http:"),
        valid.replace("sandbox-customer-portal", "customer-portal"),
        valid.replace("paddle.com", "paddle.com.evil.test"),
        valid.replace("paddle.com", "paddle.com."),
        valid.replace("https://", "https://user@"),
        valid.replace("https://", "https://:password@"),
        valid.replace("paddle.com", "paddle.com:444"),
        valid.replace(PORTAL, "wrong-path"),
        valid.replace(PORTAL, &format!("{PORTAL}/extra")),
        format!("{valid}#secret"),
        format!("{valid}\n"),
        valid.replace("/cpl_", "\\cpl_"),
        valid.replace("action=overview&", ""),
        valid.replace("overview", "cancel_subscription"),
        valid.replace(&format!("&token={TOKEN}"), ""),
        valid.replace(TOKEN, ""),
        valid.replace(TOKEN, "%0asecret"),
        valid.replace(TOKEN, "%20secret"),
        format!("{valid}&token=another"),
        format!("{valid}&%74oken=another"),
        format!("{valid}&action=overview"),
        format!("{valid}&redirect=https://evil.test"),
        format!("{valid}{}", "x".repeat(MAX_PORTAL_URL_BYTES)),
    ] {
        assert_mismatch(validate_url(&url, true).unwrap_err());
    }
    validate_url(&valid, true).unwrap();
    validate_url(&overview(false), false).unwrap();
    assert_mismatch(validate_url(&valid, false).unwrap_err());
    // Query order is not part of the credential or authority contract.
    validate_url(
        &valid.replace(
            &format!("action=overview&token={TOKEN}"),
            &format!("token={TOKEN}&action=overview"),
        ),
        true,
    )
    .unwrap();
}

#[test]
fn portal_url_bound_is_inclusive() {
    let mut valid = overview(true);
    valid.push_str(&"x".repeat(MAX_PORTAL_URL_BYTES - valid.len()));
    validate_url(&valid, true).unwrap();
    valid.push('x');
    assert_mismatch(validate_url(&valid, true).unwrap_err());
}

#[tokio::test]
async fn offline_mode_is_deterministic_bound_and_never_claims_real_access() {
    for key in ["", "mock_paddle"] {
        let provider = PaddleProvider::new(key, "mock_webhook");
        let request = provisioning();
        let customer = provider.create_customer(&request).await.unwrap();
        let first = provider
            .create_bound_customer_portal(&request, customer.id())
            .await
            .unwrap();
        let second = provider
            .create_bound_customer_portal(&request, customer.id())
            .await
            .unwrap();
        assert_eq!(first.url(), second.url());
        assert_eq!(first.id(), second.id());
        assert_eq!(first.customer_id(), customer.id());
        assert_eq!(first.sandbox(), None);
        assert!(first.is_mock());
        assert!(first.require_real().is_err());
        assert_eq!(
            Url::parse(first.url()).unwrap().host_str(),
            Some("example.invalid")
        );
        assert!(!format!("{first:?}").contains(first.url()));
        let foreign = PaddleCustomerRequest::new(
            "different_owner",
            "provision_opaque",
            "contact@example.test",
        )
        .unwrap();
        assert_mismatch(
            provider
                .create_bound_customer_portal(&foreign, customer.id())
                .await
                .unwrap_err(),
        );
        assert_mismatch(
            provider
                .create_bound_customer_portal(&request, CUSTOMER)
                .await
                .unwrap_err(),
        );
    }
}
