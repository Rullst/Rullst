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
    marker: PhantomData<T>,
}

#[test]
fn billable_derive_is_available_from_facade_and_preserves_generics() {
    let account = Account::<u8> {
        email: "owner@example.com".to_string(),
        subscription_id: Some("sub_1".to_string()),
        tier: Some("pro".to_string()),
        marker: PhantomData,
    };

    assert_eq!(account.email(), "owner@example.com");
    assert_eq!(account.subscription_id().as_deref(), Some("sub_1"));
    assert!(account.can_access("pro"));
    assert!(!account.can_access("enterprise"));
}
