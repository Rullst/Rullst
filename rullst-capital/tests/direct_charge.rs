use async_trait::async_trait;
use rullst_capital::{Billable, CapitalError, ChargeStatus, LemonSqueezyProvider, StripeProvider};

struct Account;

#[async_trait]
impl Billable for Account {
    fn email(&self) -> String {
        "owner@example.com".to_string()
    }
}

#[tokio::test]
async fn billable_charge_is_safe_deterministic_and_statically_dispatched() {
    let provider = StripeProvider::new("mock_stripe", "mock_webhook");
    let account = Account;

    let first = account
        .charge_with(&provider, 4_990, "BRL", "cus_1", "pm_1", "order_1")
        .await
        .expect("mock charge");
    let retry = account
        .charge_with(&provider, 4_990, "brl", "cus_1", "pm_1", "order_1")
        .await
        .expect("deterministic retry");

    assert_eq!(first, retry);
    assert_eq!(first.provider(), "stripe");
    assert_eq!(first.status(), ChargeStatus::Mock);
    assert!(!first.is_succeeded());
    assert_eq!(first.amount_minor(), 4_990);
    assert_eq!(first.currency(), "brl");
}

#[tokio::test]
async fn invalid_requests_and_unreviewed_provider_operations_fail_closed() {
    let account = Account;
    let stripe = StripeProvider::new("mock_stripe", "mock_webhook");
    let invalid = account
        .charge_with(&stripe, 0, "BRL", "cus_1", "pm_1", "order_1")
        .await;
    assert!(matches!(invalid, Err(CapitalError::InvalidCharge(_))));

    let lemon = LemonSqueezyProvider::new("mock_lemon", "mock_webhook");
    let unsupported = account
        .charge_with(&lemon, 4_990, "BRL", "cus_1", "pm_1", "order_1")
        .await;
    assert!(matches!(
        unsupported,
        Err(CapitalError::UnsupportedOperation(_))
    ));
}
