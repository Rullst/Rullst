use rullst_capital::{
    BillingProvider, CapitalError, CouponCode, LemonSqueezyProvider, MercadoPagoProvider,
    PaddleProvider, PolarProvider, RazorpayProvider, TrialExtension,
};

#[test]
fn public_coupon_and_relative_trial_contract_is_bounded() {
    let coupon = CouponCode::try_new("BLACK_FRIDAY-25").expect("coupon");
    assert_eq!(coupon.as_str(), "BLACK_FRIDAY-25");
    let extension = TrialExtension::from_days_at(15, 1_800_000_000).expect("extension");
    assert_eq!(extension.days(), 15);
    assert_eq!(extension.ends_at(), 1_801_296_000);
}

#[tokio::test]
async fn unreviewed_live_provider_operations_fail_explicitly() {
    let providers: Vec<Box<dyn BillingProvider>> = vec![
        Box::new(LemonSqueezyProvider::new("live_lemon", "mock_webhook")),
        Box::new(PolarProvider::new("live_polar", "mock_webhook")),
        Box::new(PaddleProvider::new("live_paddle", "mock_webhook")),
        Box::new(MercadoPagoProvider::new("live_mercadopago", "mock_webhook")),
        Box::new(RazorpayProvider::new(
            "live_razorpay",
            "live_secret",
            "mock_webhook",
        )),
    ];

    for provider in providers {
        assert!(matches!(
            provider.apply_coupon("sub_123", "BLACK_FRIDAY").await,
            Err(CapitalError::UnsupportedOperation(_))
        ));
        if provider.name() != "lemonsqueezy" {
            assert!(matches!(
                provider.extend_trial("sub_123", 1_900_000_000).await,
                Err(CapitalError::UnsupportedOperation(_))
            ));
        }
    }
}
