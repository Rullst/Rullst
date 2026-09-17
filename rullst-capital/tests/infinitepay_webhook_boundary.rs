use ring::hmac;
use rullst_capital::{BillingProvider, CapitalError, InfinitePayProvider};
use std::collections::HashMap;

#[test]
fn real_infinitepay_callbacks_cannot_establish_the_unreviewed_body_contract() {
    let provider = InfinitePayProvider::new("handle_fixture", "fixture-secret");
    // Public callback shape, plus the old parser's invented subscription shape.
    for body in [
        br#"{"invoice_slug":"abc123","amount":1000,"paid_amount":1000,"transaction_nsu":"tx","order_nsu":"order"}"#.as_slice(),
        br#"{"id":"tx","customer_id":"customer","plan_id":"plan","status":"paid"}"#.as_slice(),
    ] {
        let signature = hex::encode(hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, b"fixture-secret"), body));
        assert!(matches!(provider.verify_signature(body, &signature), Err(CapitalError::UnsupportedOperation(_))));
        for headers in [HashMap::new(), HashMap::from([("x-infinitepay-signature".into(), signature)])] {
            assert!(matches!(provider.handle_webhook(body, &headers), Err(CapitalError::UnsupportedOperation(_))));
        }
    }
}

#[test]
fn infinitepay_local_fixtures_require_explicit_mock_verification() {
    let body = br#"{"id":"tx_fixture","customer_id":"customer_fixture","plan_id":"plan_fixture","status":"paid"}"#;
    let headers = HashMap::from([("x-signature".into(), "mock_fixture".into())]);
    for api_key in ["", "mock_key"] {
        let provider = InfinitePayProvider::new(api_key, "mock_fixture");
        assert!(provider.handle_webhook(body, &headers).is_ok());
        assert!(
            provider
                .handle_webhook(
                    body,
                    &HashMap::from([("x-signature".into(), "wrong".into())])
                )
                .is_err()
        );
    }
    assert!(matches!(
        InfinitePayProvider::new("", "").handle_webhook(body, &headers),
        Err(CapitalError::ConfigurationError(_))
    ));
}
