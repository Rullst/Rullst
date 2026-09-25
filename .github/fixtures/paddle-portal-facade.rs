use rullst::capital::{PaddleCustomerRequest, PaddlePortalSession, PaddleProvider};

#[test]
fn packaged_facade_exposes_bound_portal_access_without_live_credentials() {
    let runtime = rullst::async_runtime::tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let provider = PaddleProvider::new("mock_package_key", "mock_webhook");
        let intent = PaddleCustomerRequest::new(
            "authenticated_owner",
            "persisted_provisioning_attempt",
            "owner@example.test",
        )
        .unwrap();
        let customer = provider.create_customer(&intent).await.unwrap();
        let portal: PaddlePortalSession = provider
            .create_bound_customer_portal(&intent, customer.id())
            .await
            .unwrap();
        assert_eq!(portal.customer_id(), customer.id());
        assert!(portal.is_mock());
        assert_eq!(portal.sandbox(), None);
        assert!(portal.require_real().is_err());
        assert!(portal.url().starts_with("https://example.invalid/"));
        assert!(!format!("{portal:?}").contains(portal.url()));
        let foreign = PaddleCustomerRequest::new(
            "different_owner",
            "persisted_provisioning_attempt",
            "owner@example.test",
        )
        .unwrap();
        assert!(
            provider
                .create_bound_customer_portal(&foreign, customer.id())
                .await
                .is_err()
        );
    });
}
