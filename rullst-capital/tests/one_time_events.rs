use ring::hmac;
use rullst_capital::{CapitalError, StripeProvider};
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

#[test]
fn verified_one_time_event_exposes_identity_digests_and_mode() {
    let provider = StripeProvider::new("sk_test_contract", "whsec_contract");
    let now = chrono::Utc::now().timestamp();
    let body = serde_json::to_vec(&event(
        "checkout.session.completed",
        json!({"object":"checkout.session","id":"cs_test_checkout","mode":"payment","livemode":false,"payment_intent":"pi_checkout"}),
        now,
    ))
    .unwrap();
    let verified = provider
        .verify_one_time_event(&body, &headers(&body, now))
        .unwrap();
    assert_eq!(verified.event_id(), "evt_one_time");
    assert_eq!(
        verified.payload_digest().as_slice(),
        ring::digest::digest(&ring::digest::SHA256, &body).as_ref()
    );
    // Versioned digest over mode, id, type, session and payment intent.
    assert_eq!(
        hex::encode(verified.mutation_digest()),
        "f7142cc3eb47b19dfa688850a54eb4fa06654d2571bf9ffe88f3840f77bff46e"
    );
    let debug = format!("{verified:?}");
    assert!(debug.starts_with("StripeOneTimeEvent"), "{debug}");
    assert!(debug.contains("livemode: false"), "{debug}");

    let live = StripeProvider::new("sk_live_contract", "whsec_contract");
    let mut refund = event(
        "charge.refunded",
        json!({"object":"charge","id":"ch_live","livemode":true,"payment_intent":"pi_live"}),
        now,
    );
    refund["livemode"] = json!(true);
    let body = serde_json::to_vec(&refund).unwrap();
    let verified = live
        .verify_one_time_event(&body, &headers(&body, now))
        .unwrap();
    assert!(verified.livemode());
    assert_eq!(
        hex::encode(verified.mutation_digest()),
        "727ae40bccbb48ea9b9b5d6a448cd0b5e299b8cc34863c5d62b4ddba5629fd75"
    );
}

#[test]
fn one_time_payload_size_is_checked_before_the_verification_mode() {
    let mock = StripeProvider::new("mock_key", "mock_secret");
    let headers = HashMap::new();
    let invalid = CapitalError::PayloadParseError("Invalid Stripe one-time event".into());
    let mock_refused = CapitalError::MockWebhookNotAllowed("stripe".into());
    assert_eq!(
        mock.verify_one_time_event(b"", &headers).unwrap_err(),
        invalid
    );
    assert_eq!(
        mock.verify_one_time_event(&vec![b'x'; 2_097_152], &headers)
            .unwrap_err(),
        mock_refused
    );
    assert_eq!(
        mock.verify_one_time_event(&vec![b'x'; 2_097_153], &headers)
            .unwrap_err(),
        invalid
    );
}
