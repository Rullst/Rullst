#![cfg(feature = "capital")]

use rullst::capital::{CouponCode, StripeProvider, SubscriptionHandle, TrialExtension};

#[tokio::test]
async fn umbrella_exposes_bounded_coupon_and_relative_trial_helpers() {
    let coupon = CouponCode::try_new("RETENTION_25").expect("coupon");
    let extension = TrialExtension::from_days_at(15, 1_800_000_000).expect("trial extension");
    assert_eq!(extension.ends_at(), 1_801_296_000);

    let stripe = StripeProvider::new("mock_subscription", "mock_webhook");
    let handle = SubscriptionHandle::new(&stripe, "sub_facade").expect("subscription handle");
    assert!(handle.apply_coupon(coupon.as_str()).await.is_ok());
    assert!(
        handle
            .extend_trial_days_at(extension.days(), 1_800_000_000)
            .await
            .is_ok()
    );
}
