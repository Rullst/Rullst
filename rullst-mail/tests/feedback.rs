use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit, Mac};
use rullst_mail::{
    InMemorySuppressionStore, MailFeedbackError, MailFeedbackKind, MutableSuppressionStore,
    ResendFeedbackVerifier, SuppressionReason, SuppressionStore,
};
use serde_json::json;
use sha2::Sha256;

fn sign(payload: &[u8], id: &str, now: u64) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(&[7; 32]).unwrap();
    mac.update(format!("{id}.{now}.").as_bytes());
    mac.update(payload);
    format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes()))
}
fn verifier() -> ResendFeedbackVerifier {
    ResendFeedbackVerifier::new(format!("whsec_{}", STANDARD.encode([7; 32]))).unwrap()
}
fn body(kind: &str, bounce: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"type":kind,"created_at":chrono::Utc::now().to_rfc3339(),
        "data":{"email_id":"email_opaque","to":["member@example.com"],"bounce":{"type":bounce},"subject":"NEVER_LOG_THIS"}})).unwrap()
}

#[tokio::test]
async fn authenticated_permanent_feedback_is_replay_bound_and_suppresses() {
    let now = chrono::Utc::now().timestamp() as u64;
    let payload = body("email.bounced", "Permanent");
    let verified = verifier()
        .verify(
            &payload,
            "msg_event1",
            &now.to_string(),
            &sign(&payload, "msg_event1", now),
            now,
        )
        .unwrap();
    assert_eq!(verified.kind(), MailFeedbackKind::PermanentBounce);
    assert_eq!(verified.event_id(), "msg_event1");
    assert_eq!(verified.email_id(), "email_opaque");
    assert_eq!(verified.recipient(), "member@example.com");
    assert!(!format!("{verified:?}").contains("member@example.com"));
    assert!(!format!("{verified:?}").contains("NEVER_LOG_THIS"));
    let store = InMemorySuppressionStore::new(10, 10).unwrap();
    let event = verified.suppression_event().unwrap().unwrap();
    store.record(event.clone()).await.unwrap();
    store.record(event).await.unwrap();
    assert_eq!(
        store
            .lookup("member@example.com")
            .await
            .unwrap()
            .unwrap()
            .reason(),
        SuppressionReason::HardBounce
    );
    let payload = body("email.complained", "");
    let complaint = verifier()
        .verify(
            &payload,
            "msg_complaint",
            &now.to_string(),
            &sign(&payload, "msg_complaint", now),
            now,
        )
        .unwrap();
    store
        .record(complaint.suppression_event().unwrap().unwrap())
        .await
        .unwrap();
    assert_eq!(
        store
            .lookup("member@example.com")
            .await
            .unwrap()
            .unwrap()
            .reason(),
        SuppressionReason::SpamComplaint
    );
    for (kind, bounce, expected) in [
        ("email.delivered", "", MailFeedbackKind::Delivered),
        ("email.delivery_delayed", "", MailFeedbackKind::Delayed),
        ("email.bounced", "Transient", MailFeedbackKind::Delayed),
        ("email.failed", "", MailFeedbackKind::Failed),
    ] {
        let payload = body(kind, bounce);
        let event = verifier()
            .verify(
                &payload,
                "msg_event2",
                &now.to_string(),
                &sign(&payload, "msg_event2", now),
                now,
            )
            .unwrap();
        assert_eq!(event.kind(), expected);
        assert!(event.suppression_event().unwrap().is_none());
    }
}

#[test]
fn valid_signature_does_not_authorize_malformed_provider_feedback() {
    let now = chrono::Utc::now().timestamp() as u64;
    let original: serde_json::Value = serde_json::from_slice(&body("email.delivered", "")).unwrap();
    for (pointer, invalid) in [
        ("/type", json!("email.unreviewed")),
        ("/created_at", json!("2099-01-01T00:00:00Z")),
        ("/created_at", json!("1960-01-01T00:00:00Z")),
        ("/created_at", json!("invalid")),
        ("/data/to", json!([])),
        (
            "/data/to",
            json!(["member@example.com", "other@example.com"]),
        ),
        ("/data/to", json!(["invalid\r\naddress"])),
        ("/data/email_id", json!("")),
        ("/data/email_id", json!("private@example.com")),
    ] {
        let mut value = original.clone();
        *value.pointer_mut(pointer).unwrap() = invalid;
        let payload = serde_json::to_vec(&value).unwrap();
        let result = verifier().verify(
            &payload,
            "msg_contract",
            &now.to_string(),
            &sign(&payload, "msg_contract", now),
            now,
        );
        let error = result.unwrap_err();
        assert_eq!(error, MailFeedbackError::InvalidPayload, "{pointer}");
        assert!(!error.to_string().contains("private@example.com"));
    }
    assert!(matches!(
        verifier().verify(b"", "msg_contract", &now.to_string(), "", now),
        Err(MailFeedbackError::InvalidPayload)
    ));
    for size in [1, 129] {
        assert!(matches!(
            ResendFeedbackVerifier::new(format!("whsec_{}", STANDARD.encode(vec![7; size]))),
            Err(MailFeedbackError::Configuration)
        ));
    }
}

#[test]
fn signatures_bind_raw_body_identity_and_freshness() {
    let now = chrono::Utc::now().timestamp() as u64;
    let payload = body("email.complained", "");
    let signature = sign(&payload, "msg_event", now);
    let verifier = verifier();
    assert!(
        verifier
            .verify(
                &payload,
                "msg_event",
                &now.to_string(),
                &format!("v1,invalid {signature}"),
                now
            )
            .is_ok()
    );
    let mut altered = payload.clone();
    altered.push(b' ');
    for (body, id, time, signed) in [
        (&altered[..], "msg_event", now, signature.as_str()),
        (&payload[..], "msg_other", now, signature.as_str()),
        (&payload[..], "msg_event", now + 1, signature.as_str()),
        (&payload[..], "msg_event", now, "v1,invalid"),
    ] {
        assert!(
            verifier
                .verify(body, id, &time.to_string(), signed, now)
                .is_err()
        );
    }
    assert!(matches!(
        verifier.verify(
            &payload,
            "msg_event",
            &now.to_string(),
            &signature,
            now + 301
        ),
        Err(MailFeedbackError::Stale)
    ));
    assert!(matches!(
        ResendFeedbackVerifier::new("").unwrap().verify(
            &payload,
            "msg_event",
            &now.to_string(),
            &signature,
            now
        ),
        Err(MailFeedbackError::MockNotAllowed)
    ));
    assert!(ResendFeedbackVerifier::new("whsec_bad").is_err());
}
