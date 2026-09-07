//! Header-authenticated real HMAC fixtures; no provider network calls.
use base64::{Engine, engine::general_purpose::STANDARD};
use ring::hmac;
use rullst_capital::{
    CapitalError,
    providers::{BillingProvider, PolarProvider, SubscriptionStatus},
};
use std::collections::HashMap;

const PAYLOAD: &[u8] = br#"{"type":"subscription.updated","data":{"id":"sub_1","customer_id":"cus_1","product_id":"prod_1","status":"active"}}"#;

fn headers(key: &[u8], payload: &[u8], timestamp: i64) -> HashMap<String, String> {
    let id = "evt_fixture";
    let mut context = hmac::Context::with_key(&hmac::Key::new(hmac::HMAC_SHA256, key));
    context.update(format!("{id}.{timestamp}.").as_bytes());
    context.update(payload);
    HashMap::from([
        ("webhook-id".into(), id.into()),
        ("webhook-timestamp".into(), timestamp.to_string()),
        (
            "webhook-signature".into(),
            format!("v1,{}", STANDARD.encode(context.sign().as_ref())),
        ),
    ])
}

#[test]
fn polar_standard_headers_bind_body_id_time_and_documented_key_schemes() {
    let now = chrono::Utc::now().timestamp();
    let key = b"a-long-deterministic-signing-key-fixture";
    let standard_secret = format!("whsec_{}", STANDARD.encode(key));
    for (secret, signing_key) in [
        ("legacy-literal-secret", b"legacy-literal-secret".as_slice()),
        (standard_secret.as_str(), key.as_slice()),
    ] {
        let provider = PolarProvider::new("fixture-key", secret);
        let valid = headers(signing_key, PAYLOAD, now);
        assert_eq!(
            provider.handle_webhook(PAYLOAD, &valid).unwrap().status,
            SubscriptionStatus::Active
        );
        assert!(matches!(
            provider.verify_signature(PAYLOAD, "body-only-hex"),
            Err(CapitalError::UnsupportedOperation(_))
        ));
        for removed in ["webhook-id", "webhook-timestamp", "webhook-signature"] {
            let mut missing = valid.clone();
            missing.remove(removed);
            assert!(provider.handle_webhook(PAYLOAD, &missing).is_err());
        }
        for (name, value) in [
            ("webhook-id", "other"),
            ("webhook-id", "evt_fixture,other"),
            ("webhook-timestamp", "not-a-time"),
            ("webhook-signature", "v1,invalid!"),
            ("webhook-signature", "v2,AAAA"),
        ] {
            let mut altered = valid.clone();
            altered.insert(name.into(), value.into());
            assert!(provider.handle_webhook(PAYLOAD, &altered).is_err());
        }
        let mut duplicate = valid.clone();
        duplicate.insert("Webhook-Id".into(), "evt_fixture".into());
        assert!(provider.handle_webhook(PAYLOAD, &duplicate).is_err());
        assert!(provider.handle_webhook(b"{}", &valid).is_err());
        for timestamp in [now - 301, now + 301] {
            assert!(
                provider
                    .handle_webhook(PAYLOAD, &headers(signing_key, PAYLOAD, timestamp))
                    .is_err()
            );
        }
        let mut rotated = valid.clone();
        rotated.insert(
            "webhook-signature".into(),
            format!(
                "v1,{} {}",
                STANDARD.encode([0; 32]),
                valid["webhook-signature"]
            ),
        );
        assert!(provider.handle_webhook(PAYLOAD, &rotated).is_ok());
        assert!(
            provider
                .handle_webhook(b"{}", &headers(signing_key, b"{}", now))
                .is_err()
        );
    }
}
