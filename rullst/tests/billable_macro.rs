#![cfg(feature = "capital")]

use std::marker::PhantomData;

use rullst::capital::Billable as _;

#[derive(rullst::Billable)]
struct Account<T>
where
    T: Clone + Send + Sync,
{
    email: String,
    subscription_id: Option<String>,
    tier: Option<String>,
    grace_period_starts_at: Option<i64>,
    grace_period_ends_at: Option<i64>,
    marker: PhantomData<T>,
}

#[test]
fn billable_derive_is_available_from_facade_and_preserves_generics() {
    let account = Account::<u8> {
        email: "owner@example.com".to_string(),
        subscription_id: Some("sub_1".to_string()),
        tier: Some("pro".to_string()),
        grace_period_starts_at: Some(1_000),
        grace_period_ends_at: Some(1_100),
        marker: PhantomData,
    };

    assert_eq!(account.email(), "owner@example.com");
    assert_eq!(account.subscription_id().as_deref(), Some("sub_1"));
    assert!(account.can_access("pro"));
    assert!(!account.can_access("enterprise"));
    assert!(account.grace_period().unwrap().is_some());
}

#[tokio::test]
async fn billable_derive_exposes_the_bounded_direct_charge_helper() {
    let account = Account::<u8> {
        email: "owner@example.com".to_string(),
        subscription_id: None,
        tier: None,
        grace_period_starts_at: None,
        grace_period_ends_at: None,
        marker: PhantomData,
    };
    let provider = rullst::capital::StripeProvider::new("mock_stripe", "mock_webhook");
    let receipt = account
        .charge_with(&provider, 1_099, "USD", "cus_1", "pm_1", "order_1")
        .await
        .expect("mock direct charge");

    assert_eq!(receipt.status(), rullst::capital::ChargeStatus::Mock);
    assert!(!receipt.is_succeeded());
    assert_eq!(receipt.amount_minor(), 1_099);
    assert_eq!(receipt.currency(), "usd");
}
