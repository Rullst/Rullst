use ring::hmac;
use rullst_capital::StripeProvider;
use serde_json::{Value, json};
use std::collections::HashMap;

fn headers(payload: &[u8], now: i64) -> HashMap<String, String> {
    let mut message = format!("{now}.").into_bytes();
    message.extend_from_slice(payload);
    let signature = hmac::sign(
        &hmac::Key::new(hmac::HMAC_SHA256, b"whsec_contract"),
        &message,
    );
    let hex: String = signature
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    HashMap::from([("stripe-signature".into(), format!("t={now},v1={hex}"))])
}
fn event(kind: &str, object: Value, now: i64) -> Value {
    json!({"id":"evt_one_time","object":"event","type":kind,"created":now,
        "livemode":false,"data":{"object":object}})
}

#[test]
fn payment_refund_and_dispute_hints_require_exact_signature_and_mode() {
    let provider = StripeProvider::new("sk_test_contract", "whsec_contract");
    let now = chrono::Utc::now().timestamp();
    for (kind, object, session) in [
        (
            "checkout.session.completed",
            json!({"object":"checkout.session","id":"cs_test_checkout","mode":"payment","livemode":false,"payment_intent":"pi_checkout"}),
            Some("cs_test_checkout"),
        ),
        (
            "charge.refunded",
            json!({"object":"charge","id":"ch_checkout","livemode":false,"payment_intent":"pi_checkout"}),
            None,
        ),
        (
            "charge.dispute.created",
            json!({"object":"dispute","id":"dp_checkout","livemode":false,"payment_intent":"pi_checkout"}),
            None,
        ),
    ] {
        let body = serde_json::to_vec(&event(kind, object, now)).unwrap();
        let signed = headers(&body, now);
        let verified = provider.verify_one_time_event(&body, &signed).unwrap();
        assert_eq!(verified.session_id(), session);
        assert_eq!(verified.payment_intent_id(), Some("pi_checkout"));
        assert_eq!(verified.event_type(), kind);
        assert!(!verified.livemode());
        assert!(!format!("{verified:?}").contains("pi_checkout"));
        let mut tampered = body.clone();
        tampered.push(b' ');
        assert!(provider.verify_one_time_event(&tampered, &signed).is_err());
        assert!(
            provider
                .verify_one_time_event(&body, &headers(&body, now - 301))
                .is_err()
        );
        assert!(
            StripeProvider::new("sk_live_contract", "whsec_contract")
                .verify_one_time_event(&body, &signed)
                .is_err()
        );
        assert!(
            StripeProvider::new("", "")
                .verify_one_time_event(&body, &signed)
                .is_err()
        );
    }
}

#[test]
fn subscription_connected_account_and_inconsistent_payloads_are_rejected() {
    let provider = StripeProvider::new("sk_test_contract", "whsec_contract");
    let now = chrono::Utc::now().timestamp();
    let base = event(
        "checkout.session.completed",
        json!({"object":"checkout.session","id":"cs_test_checkout","mode":"payment","livemode":false,"payment_intent":"pi_checkout"}),
        now,
    );
    for mutation in 0..6 {
        let mut value = base.clone();
        match mutation {
            0 => value["data"]["object"]["mode"] = json!("subscription"),
            1 => value["data"]["object"]["subscription"] = json!("sub_other"),
            2 => value["account"] = json!("acct_connect"),
            3 => value["data"]["object"]["livemode"] = json!(true),
            4 => value["type"] = json!("unrecognized.event"),
            _ => value["id"] = json!("<script>"),
        }
        let body = serde_json::to_vec(&value).unwrap();
        assert!(
            provider
                .verify_one_time_event(&body, &headers(&body, now))
                .is_err()
        );
    }
}
