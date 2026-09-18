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
