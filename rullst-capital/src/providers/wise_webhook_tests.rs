use super::*;
use rcgen::{KeyPair, PKCS_ECDSA_P256_SHA256, PKCS_RSA_SHA256, PublicKeyData, SigningKey};
use std::sync::OnceLock;

// Wise's published sandbox example from
// https://github.com/transferwise/digital-signatures-examples
// (verify-webhook-signature/verify-signature.js): a public key, a delivered
// body and its signature. It proves interoperability with Wise's own scheme.
const WISE_EXAMPLE_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAwpb91cEYuyJNQepZAVfP
ZIlPZfNUefH+n6w9SW3fykqKu938cR7WadQv87oF2VuT+fDt7kqeRziTmPSUhqPU
ys/V2Q1rlfJuXbE+Gga37t7zwd0egQ+KyOEHQOpcTwKmtZ81ieGHynAQzsn1We3j
wt760MsCPJ7GMT141ByQM+yW1Bx+4SG3IGjXWyqOWrcXsxAvIXkpUD/jK/L958Cg
nZEgz0BSEh0QxYLITnW1lLokSx/dTianWPFEhMC9BgijempgNXHNfcVirg1lPSyg
z7KqoKUN0oHqWLr2U1A+7kqrl6O2nx3CKs1bj1hToT1+p4kcMoHXA7kA+VBLUpEs
VwIDAQAB
-----END PUBLIC KEY-----";
const WISE_EXAMPLE_BODY: &str = r#"{"data":{"resource":{"id":49983981,"profile_id":16055450,"account_id":14124090,"type":"transfer"},"current_state":"incoming_payment_waiting","previous_state":null,"occurred_at":"2021-08-23T10:12:50Z"},"subscription_id":"90aa8e14-4ef1-4a56-861c-f3c9cde097ea","event_type":"transfers#state-change","schema_version":"2.0.0","sent_at":"2021-08-23T10:12:50Z"}"#;
const WISE_EXAMPLE_SIGNATURE: &str = "wKcKCYXAzxNgiu7xmoDm943NUni7Rz33QN8JkEA9dWSGebgndonabgSj18Y4C08OrwVmueGsED2s00M7DtJVcYKOS1i3G4TMVx+mgM3aL9djMBkQtiYNBFUd6wrPI7ZUNHv/TrlKSjTMc+6JFvUvJ7owY3z85e3I4jLRLJowMFvO8kvCJ60+1pY9wDwZvtZ//WS93LrwGjk9Dvwzpmu0w+P4J75tETT5qC3Uv0y5G2yO8SEoO3yNP/tg/BOli02niHb53vEOUWUb9bly6thnfMoXoiV/osoGxgF20R58RlvkAmezyyl1Sv542TfS2DpiwVnmjjjkCyXeSUcKookYLQ==";

fn signature_headers(signature: &str) -> HashMap<String, String> {
    HashMap::from([("x-signature-sha256".to_string(), signature.to_string())])
}

fn wise_example_provider() -> WiseProvider {
    WiseProvider::new("mock_wise_token", "16055450")
        .with_webhook_public_key_pem(WISE_EXAMPLE_PUBLIC_KEY)
        .unwrap()
}

fn pem(label: &str, der: &[u8]) -> String {
    let encoded = STANDARD.encode(der);
    let lines: Vec<&str> = encoded
        .as_bytes()
        .chunks(64)
        .map(|line| std::str::from_utf8(line).unwrap())
        .collect();
    format!(
        "-----BEGIN {label}-----\n{}\n-----END {label}-----\n",
        lines.join("\n")
    )
}

// One locally generated RSA key signs synthetic deliveries for parser tests.
fn local_signer() -> &'static KeyPair {
    static SIGNER: OnceLock<KeyPair> = OnceLock::new();
    SIGNER.get_or_init(|| KeyPair::generate_for(&PKCS_RSA_SHA256).unwrap())
}

fn local_provider() -> WiseProvider {
    let public_key = pem("PUBLIC KEY", &local_signer().subject_public_key_info());
    WiseProvider::new("", "profile")
        .with_webhook_public_key_pem(public_key)
        .unwrap()
}

fn signed(body: &Value) -> (Vec<u8>, HashMap<String, String>) {
    let bytes = serde_json::to_vec(body).unwrap();
    let signature = STANDARD.encode(local_signer().sign(&bytes).unwrap());
    (bytes, signature_headers(&signature))
}

fn state_change(state: &str) -> Value {
    serde_json::json!({
        "data": {
            "resource": {"id": 111, "profile_id": 222, "account_id": 333, "type": "transfer"},
            "current_state": state,
            "previous_state": "processing",
            "occurred_at": "2026-09-29T12:00:00Z"
        },
        "subscription_id": "01234567-89ab-cdef-0123-456789abcdef",
        "event_type": "transfers#state-change",
        "schema_version": "2.0.0",
        "sent_at": "2026-09-29T12:00:01Z"
    })
}

#[test]
fn wise_published_sandbox_delivery_verifies_and_normalizes() {
    let event = wise_example_provider()
        .verify_transfer_state_change(
            WISE_EXAMPLE_BODY.as_bytes(),
            &signature_headers(WISE_EXAMPLE_SIGNATURE),
        )
        .unwrap();
    assert_eq!(event.transfer_id(), 49_983_981);
    assert_eq!(event.profile_id(), Some(16_055_450));
    assert_eq!(
        event.current_state(),
        WiseTransferState::IncomingPaymentWaiting
    );
    assert_eq!(event.previous_state(), None);
    assert_eq!(event.occurred_at(), 1_629_713_570);
    assert_eq!(
        event.current_state().payout_status(),
        Some(PayoutStatus::Processing)
    );
}

#[test]
fn forged_tampered_or_unsigned_deliveries_are_rejected_before_parsing() {
    let provider = wise_example_provider();
    let tampered = WISE_EXAMPLE_BODY.replace("incoming_payment_waiting", "outgoing_payment_sent");
    let mut flipped = STANDARD.decode(WISE_EXAMPLE_SIGNATURE).unwrap();
    flipped[10] ^= 1;
    for (body, headers) in [
        (tampered.as_str(), signature_headers(WISE_EXAMPLE_SIGNATURE)),
        (
            WISE_EXAMPLE_BODY,
            signature_headers(&STANDARD.encode(&flipped)),
        ),
        (WISE_EXAMPLE_BODY, signature_headers("not base64!")),
        (WISE_EXAMPLE_BODY, signature_headers("   ")),
        (WISE_EXAMPLE_BODY, signature_headers(&"A".repeat(1_404))),
        (WISE_EXAMPLE_BODY, HashMap::new()),
        (
            WISE_EXAMPLE_BODY,
            HashMap::from([(
                "x-signature".to_string(),
                WISE_EXAMPLE_SIGNATURE.to_string(),
            )]),
        ),
    ] {
        assert!(matches!(
            provider.verify_transfer_state_change(body.as_bytes(), &headers),
            Err(CapitalError::InvalidSignature(_))
        ));
    }
    // A correctly signed body still needs a configured, matching key.
    assert!(matches!(
        local_provider().verify_transfer_state_change(
            WISE_EXAMPLE_BODY.as_bytes(),
            &signature_headers(WISE_EXAMPLE_SIGNATURE)
        ),
        Err(CapitalError::InvalidSignature(_))
    ));
    assert!(matches!(
        WiseProvider::new("mock_wise_token", "profile").verify_transfer_state_change(
            WISE_EXAMPLE_BODY.as_bytes(),
            &signature_headers(WISE_EXAMPLE_SIGNATURE)
        ),
        Err(CapitalError::ConfigurationError(_))
    ));
    let oversized = vec![b' '; MAX_BODY_BYTES + 1];
    assert!(matches!(
        provider
            .verify_transfer_state_change(&oversized, &signature_headers(WISE_EXAMPLE_SIGNATURE)),
        Err(CapitalError::PayloadParseError(_))
    ));
}

#[test]
fn rotated_keys_are_accepted_up_to_the_configured_bound() {
    let public_key = pem("PUBLIC KEY", &local_signer().subject_public_key_info());
    let rotated = local_provider()
        .with_webhook_public_key_pem(WISE_EXAMPLE_PUBLIC_KEY)
        .unwrap();
    assert!(
        rotated
            .verify_transfer_state_change(
                WISE_EXAMPLE_BODY.as_bytes(),
                &signature_headers(WISE_EXAMPLE_SIGNATURE)
            )
            .is_ok()
    );
    let (body, headers) = signed(&state_change("processing"));
    assert!(
        rotated
            .verify_transfer_state_change(&body, &headers)
            .is_ok()
    );
    // Two keys are configured; two more reach the bound of four.
    let full = rotated
        .with_webhook_public_key_pem(&public_key)
        .unwrap()
        .with_webhook_public_key_pem(&public_key)
        .unwrap();
    assert!(matches!(
        full.with_webhook_public_key_pem(&public_key),
        Err(CapitalError::ConfigurationError(_))
    ));
}

#[test]
fn every_documented_state_is_typed_without_inventing_values() {
    let provider = local_provider();
    for (state, typed, legacy) in [
        (
            "incoming_payment_waiting",
            WiseTransferState::IncomingPaymentWaiting,
            Some(PayoutStatus::Processing),
        ),
        (
            "incoming_payment_initiated",
            WiseTransferState::IncomingPaymentInitiated,
            Some(PayoutStatus::Processing),
        ),
        (
            "processing",
            WiseTransferState::Processing,
            Some(PayoutStatus::Processing),
        ),
        (
            "funds_converted",
            WiseTransferState::FundsConverted,
            Some(PayoutStatus::Processing),
        ),
        (
            "outgoing_payment_sent",
            WiseTransferState::OutgoingPaymentSent,
            Some(PayoutStatus::OutgoingPaymentSent),
        ),
        ("charged_back", WiseTransferState::ChargedBack, None),
        (
            "cancelled",
            WiseTransferState::Cancelled,
            Some(PayoutStatus::Cancelled),
        ),
        (
            "funds_refunded",
            WiseTransferState::FundsRefunded,
            Some(PayoutStatus::FundsRefunded),
        ),
        ("bounced_back", WiseTransferState::BouncedBack, None),
    ] {
        let (body, headers) = signed(&state_change(state));
        let event = provider
            .verify_transfer_state_change(&body, &headers)
            .unwrap();
        assert_eq!(event.current_state(), typed);
        assert_eq!(typed.as_str(), state);
        assert_eq!(typed.payout_status(), legacy);
        assert_eq!(event.transfer_id(), 111);
        assert_eq!(event.profile_id(), Some(222));
        assert_eq!(event.previous_state(), Some(WiseTransferState::Processing));
    }
    let mut without_profile = state_change("processing");
    without_profile["data"]["resource"]
        .as_object_mut()
        .unwrap()
        .remove("profile_id");
    let (body, headers) = signed(&without_profile);
    let event = provider
        .verify_transfer_state_change(&body, &headers)
        .unwrap();
    assert_eq!(event.profile_id(), None);
}

#[test]
fn signed_deliveries_missing_or_confusing_required_fields_are_rejected() {
    let provider = local_provider();
    let cases = [
        ("/data/resource/id", None),
        ("/data/resource/id", Some(Value::from(0))),
        ("/data/resource/id", Some(Value::from(-5))),
        ("/data/resource/id", Some(Value::from("111"))),
        ("/data/resource/id", Some(Value::from(1.5))),
        ("/data/resource/type", Some(Value::from("balance"))),
        ("/data/resource/type", None),
        ("/data/resource/profile_id", Some(Value::from("222"))),
        ("/data/current_state", None),
        ("/data/current_state", Some(Value::from("unknown"))),
        (
            "/data/current_state",
            Some(Value::from("OUTGOING_PAYMENT_SENT")),
        ),
        ("/data/previous_state", Some(Value::from("teleported"))),
        ("/data/occurred_at", None),
        ("/data/occurred_at", Some(Value::from("yesterday"))),
        ("/event_type", Some(Value::from("balances#credit"))),
        ("/event_type", None),
    ];
    for (pointer, replacement) in cases {
        let mut body = state_change("outgoing_payment_sent");
        let (parent, field) = pointer.rsplit_once('/').unwrap();
        let target = if parent.is_empty() {
            &mut body
        } else {
            body.pointer_mut(parent).unwrap()
        };
        match replacement {
            Some(value) => target[field] = value,
            None => {
                target.as_object_mut().unwrap().remove(field);
            }
        }
        let (bytes, headers) = signed(&body);
        assert!(
            matches!(
                provider.verify_transfer_state_change(&bytes, &headers),
                Err(CapitalError::PayloadParseError(_))
            ),
            "{pointer}"
        );
    }
    let not_json = b"not json";
    let signature = STANDARD.encode(local_signer().sign(not_json).unwrap());
    assert!(matches!(
        provider.verify_transfer_state_change(not_json, &signature_headers(&signature)),
        Err(CapitalError::PayloadParseError(_))
    ));
}

#[test]
fn only_rsa_subject_public_key_info_pem_is_accepted() {
    let rsa_spki = local_signer().subject_public_key_info();
    let ec_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
    let mut trailing = rsa_spki.clone();
    trailing.push(0);
    let mut truncated = rsa_spki.clone();
    truncated.pop();
    let rsa_public_key = local_signer().der_bytes().to_vec();
    for candidate in [
        String::new(),
        "not a key".to_string(),
        pem("PUBLIC KEY", &ec_key.subject_public_key_info()),
        pem("RSA PUBLIC KEY", &rsa_public_key),
        pem("PUBLIC KEY", &rsa_public_key),
        pem("PUBLIC KEY", &trailing),
        pem("PUBLIC KEY", &truncated),
        pem("CERTIFICATE", &rsa_spki),
        "-----BEGIN PUBLIC KEY-----\n@@@@\n-----END PUBLIC KEY-----".to_string(),
        format!("{}{}", " ".repeat(MAX_PEM_BYTES), WISE_EXAMPLE_PUBLIC_KEY),
    ] {
        assert!(matches!(
            WiseProvider::new("", "profile").with_webhook_public_key_pem(&candidate),
            Err(CapitalError::ConfigurationError(_))
        ));
    }
}

#[test]
fn der_integers_and_lengths_must_be_minimal() {
    assert_eq!(positive_integer_bits(&[0x01]), Some(1));
    assert_eq!(positive_integer_bits(&[0x00, 0x80]), Some(8));
    assert_eq!(positive_integer_bits(&[0x01, 0x00, 0x01]), Some(17));
    assert_eq!(positive_integer_bits(&[0x00, 0x7f]), None);
    assert_eq!(positive_integer_bits(&[0x80]), None);
    assert_eq!(positive_integer_bits(&[0x00]), None);
    assert_eq!(positive_integer_bits(&[]), None);
    assert!(element(&[0x30, 0x01, 0xaa], 0x30).is_some());
    assert!(element(&[0x30, 0x81, 0x01, 0xaa], 0x30).is_none());
    assert!(element(&[0x30, 0x82, 0x00, 0x81], 0x30).is_none());
    assert!(element(&[0x30, 0x80], 0x30).is_none());
    assert!(element(&[0x30, 0x02, 0xaa], 0x30).is_none());
    assert!(element(&[0x31, 0x01, 0xaa], 0x30).is_none());
}
