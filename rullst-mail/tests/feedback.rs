use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit, Mac};
use rullst_mail::{
    InMemorySuppressionStore, MailFeedbackError, MailFeedbackKind, MutableSuppressionStore,
    ResendFeedbackVerifier, SuppressionReason, SuppressionStore,
};
use serde_json::json;
use sha2::Sha256;

fn sign(key: &[u8], payload: &[u8], id: &str, now: u64) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
    mac.update(format!("{id}.{now}.").as_bytes());
    mac.update(payload);
    format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes()))
}
fn verifier(key: &[u8]) -> ResendFeedbackVerifier {
    ResendFeedbackVerifier::new(format!("whsec_{}", STANDARD.encode(key))).unwrap()
}
fn body(kind: &str, bounce: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"type":kind,"created_at":jiff::Timestamp::now().to_string(),
        "data":{"email_id":"email_opaque","to":["member@example.com"],"bounce":{"type":bounce},"subject":"NEVER_LOG_THIS"}})).unwrap()
}

#[tokio::test]
async fn authenticated_permanent_feedback_is_replay_bound_and_suppresses() {
    let key: [u8; 32] = rand::random();
    let now = jiff::Timestamp::now().as_second() as u64;
    let payload = body("email.bounced", "Permanent");
    let verified = verifier(&key)
        .verify(
            &payload,
            "msg_event1",
            &now.to_string(),
            &sign(&key, &payload, "msg_event1", now),
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
    let complaint = verifier(&key)
        .verify(
            &payload,
            "msg_complaint",
            &now.to_string(),
            &sign(&key, &payload, "msg_complaint", now),
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
        let event = verifier(&key)
            .verify(
                &payload,
                "msg_event2",
                &now.to_string(),
                &sign(&key, &payload, "msg_event2", now),
                now,
            )
            .unwrap();
        assert_eq!(event.kind(), expected);
        assert!(event.suppression_event().unwrap().is_none());
    }
}

#[test]
fn valid_signature_does_not_authorize_malformed_provider_feedback() {
    let key: [u8; 32] = rand::random();
    let now = jiff::Timestamp::now().as_second() as u64;
    let original: serde_json::Value =
        serde_json::from_slice(&body("email.bounced", "Permanent")).unwrap();
    for (pointer, invalid) in [
        ("/type", json!(7)),
        ("/data/bounce/type", json!("Unreviewed")),
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
        let result = verifier(&key).verify(
            &payload,
            "msg_contract",
            &now.to_string(),
            &sign(&key, &payload, "msg_contract", now),
            now,
        );
        let error = result.unwrap_err();
        assert_eq!(error, MailFeedbackError::InvalidPayload, "{pointer}");
        assert!(!error.to_string().contains("private@example.com"));
    }
    assert!(matches!(
        verifier(&key).verify(b"", "msg_contract", &now.to_string(), "", now),
        Err(MailFeedbackError::InvalidPayload)
    ));
    for size in [1, 129] {
        assert!(matches!(
            ResendFeedbackVerifier::new(format!(
                "whsec_{}",
                STANDARD.encode((0..size).map(|_| rand::random::<u8>()).collect::<Vec<_>>())
            )),
            Err(MailFeedbackError::Configuration)
        ));
    }
}

#[test]
fn signatures_bind_raw_body_identity_and_freshness() {
    let key: [u8; 32] = rand::random();
    let now = jiff::Timestamp::now().as_second() as u64;
    let payload = body("email.complained", "");
    let signature = sign(&key, &payload, "msg_event", now);
    let mut other_key = key;
    other_key[0] ^= 1;
    assert!(matches!(
        verifier(&other_key).verify(&payload, "msg_event", &now.to_string(), &signature, now),
        Err(MailFeedbackError::InvalidSignature)
    ));
    let verifier = verifier(&key);
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

#[test]
fn display_name_feedback_recipient_is_reduced_to_the_bare_address() {
    let key: [u8; 32] = rand::random();
    let now = jiff::Timestamp::now().as_second() as u64;
    let payload = serde_json::to_vec(&json!({"type":"email.complained",
        "created_at":jiff::Timestamp::now().to_string(),
        "data":{"email_id":"email_opaque","to":["Member <member@example.com>"]}}))
    .unwrap();
    let verified = verifier(&key)
        .verify(
            &payload,
            "msg_named",
            &now.to_string(),
            &sign(&key, &payload, "msg_named", now),
            now,
        )
        .unwrap();
    assert_eq!(verified.recipient(), "member@example.com");
    let event = verified.suppression_event().unwrap().unwrap();
    assert_eq!(event.recipient(), "member@example.com");
}

#[test]
fn authentic_unhandled_event_types_are_distinguishable_from_malformed_ones() {
    let key: [u8; 32] = rand::random();
    let now = jiff::Timestamp::now().as_second() as u64;
    for kind in [
        "email.sent",
        "email.opened",
        "email.clicked",
        "contact.created",
    ] {
        let payload = body(kind, "");
        let signature = sign(&key, &payload, "msg_ignored", now);
        assert_eq!(
            verifier(&key)
                .verify(&payload, "msg_ignored", &now.to_string(), &signature, now)
                .unwrap_err(),
            MailFeedbackError::UnsupportedEvent,
            "{kind}"
        );
        // Without a valid signature the event is never called authentic.
        let forged = sign(&[7_u8; 32], &payload, "msg_ignored", now);
        assert_eq!(
            verifier(&key)
                .verify(&payload, "msg_ignored", &now.to_string(), &forged, now)
                .unwrap_err(),
            MailFeedbackError::InvalidSignature
        );
    }
}

/// Signs an exact timestamp text, which may carry leading zeros.
fn sign_text(key: &[u8], payload: &[u8], id: &str, timestamp: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
    mac.update(format!("{id}.{timestamp}.").as_bytes());
    mac.update(payload);
    format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes()))
}

/// 2026-01-01T00:00:00Z.
const BASE: u64 = 1_767_225_600;

fn complaint(created_at: &str, recipient: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"type":"email.complained","created_at":created_at,
        "data":{"email_id":"email_opaque","to":[recipient]}}))
    .unwrap()
}

#[test]
fn verification_bounds_are_inclusive_and_errors_have_fixed_text() {
    let key: [u8; 32] = rand::random();
    let verifier = verifier(&key);
    let verify = |payload: &[u8], id: &str, timestamp: &str, now: u64| {
        let signature = sign_text(&key, payload, id, timestamp);
        verifier.verify(payload, id, timestamp, &signature, now)
    };
    let invalid = Err(MailFeedbackError::InvalidPayload);
    let payload = complaint("2026-01-01T00:00:00Z", "member@example.com");

    let verified = verify(&payload, "msg_bounds", "1767225600", BASE).unwrap();
    assert_eq!(
        format!("{verified:?}"),
        "VerifiedMailFeedback { kind: Complaint, observed_at: 1767225600, .. }"
    );
    assert_eq!(
        MailFeedbackError::Stale.to_string(),
        "mail feedback rejected: Stale"
    );

    // The raw body holds at most 1 MiB; trailing JSON whitespace pads it.
    let padded = |length: usize| {
        let mut body = payload.clone();
        body.resize(length, b' ');
        body
    };
    assert!(verify(&padded(1_048_576), "msg_bounds", "1767225600", BASE).is_ok());
    let oversized = verify(&padded(1_048_577), "msg_bounds", "1767225600", BASE);
    assert_eq!(oversized.map(|_| ()), invalid);

    // Identifiers are checked even when the signature covers them.
    let dotted = verify(&payload, "msg.bounds", "1767225600", BASE);
    assert_eq!(dotted.map(|_| ()), invalid);

    // The timestamp text holds at most 20 bytes.
    assert!(verify(&payload, "msg_bounds", "00000000001767225600", BASE).is_ok());
    let long_time = verify(&payload, "msg_bounds", "000000000001767225600", BASE);
    assert_eq!(long_time.map(|_| ()), invalid);

    // The signature header holds at most 1024 bytes.
    let signature = sign_text(&key, &payload, "msg_bounds", "1767225600");
    let header = |length: usize| {
        let filler = "x".repeat(length - 1 - signature.len());
        format!("{filler} {signature}")
    };
    for (length, accepted) in [(1_024, true), (1_025, false)] {
        let result = verifier.verify(&payload, "msg_bounds", "1767225600", &header(length), BASE);
        assert_eq!(result.is_ok(), accepted, "{length}");
    }

    // Signatures 300 s old and events created 300 s ahead are still accepted.
    assert!(verify(&payload, "msg_bounds", "1767225300", BASE).is_ok());
    let ahead = complaint("2026-01-01T00:05:00Z", "member@example.com");
    assert!(verify(&ahead, "msg_bounds", "1767225600", BASE).is_ok());

    // The bare recipient address holds at most 254 bytes.
    let recipient = |local: usize| format!("{}@example.com", "a".repeat(local));
    let longest = complaint("2026-01-01T00:00:00Z", &recipient(242));
    assert!(verify(&longest, "msg_bounds", "1767225600", BASE).is_ok());
    let too_long = complaint("2026-01-01T00:00:00Z", &recipient(243));
    let result = verify(&too_long, "msg_bounds", "1767225600", BASE);
    assert_eq!(result.map(|_| ()), invalid);
}
