use rullst::capital::{StripeCustomerRequest, StripeCustomerStatus, StripeProvider};

#[test]
fn packaged_facade_exposes_bound_portal_access_without_live_credentials() {
    let runtime = rullst::async_runtime::tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let provider = StripeProvider::new("mock_package_key", "mock_webhook");
        let intent = StripeCustomerRequest::new("authenticated_owner", "persisted_attempt")
            .unwrap()
            .with_email("owner@example.test")
            .unwrap();
        let customer = provider.create_customer(&intent).await.unwrap();
        assert_eq!(customer.status(), StripeCustomerStatus::Mock);
        let portal = provider
            .create_bound_customer_portal(customer.id(), "https://app.example.test/billing")
            .await
            .unwrap();
        assert!(portal.starts_with("https://mock.stripe.invalid/portal/cus_mock_"));
        assert!(
            provider
                .create_bound_customer_portal("not_a_customer", "https://app.example.test/billing")
                .await
                .is_err()
        );
    });
}
