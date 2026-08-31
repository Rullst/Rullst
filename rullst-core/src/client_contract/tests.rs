#![allow(clippy::expect_used)]

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LessonAttempt {
    lesson_id: String,
    answer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LessonResult {
    correct: bool,
    awarded_points: u16,
}

fn version(value: u16) -> ContractVersion {
    ContractVersion::new(value).expect("positive test version")
}

fn request_id() -> RequestId {
    RequestId::new("req_01j8lesson").expect("valid test request id")
}

#[test]
fn identifiers_and_versions_fail_closed() {
    assert!(ContractVersion::new(0).is_err());
    assert!(RequestId::new("").is_err());
    assert!(RequestId::new("request/escape").is_err());
    assert!(RequestId::new("é").is_err());
    assert!(IdempotencyKey::new("short").is_err());
    assert!(IdempotencyKey::new("attempt_01j8lesson").is_ok());
    assert!(FailureCode::new("lesson.answer_invalid").is_ok());
    assert!(FailureCode::new("Lesson.Invalid").is_err());
    assert!(FailureCode::new("lesson..invalid").is_err());
}

#[test]
fn negotiation_is_bounded_canonical_and_selects_highest_mutual_version() {
    let offer = ClientVersionOffer::new([version(3), version(1), version(2), version(2)])
        .expect("bounded version offer");
    assert_eq!(
        offer.supported_versions(),
        &[version(1), version(2), version(3)]
    );

    let policy =
        ClientContractPolicy::new(version(1), version(2), 1_024).expect("valid contract policy");
    let encoded = policy.encode_offer(&offer).expect("encoded offer");
    assert_eq!(policy.decode_offer(&encoded).expect("decoded offer"), offer);
    assert_eq!(
        policy.negotiate(&offer).expect("highest mutual version"),
        version(2)
    );

    let unsupported = ClientVersionOffer::new([version(3)]).expect("bounded version offer");
    assert!(matches!(
        policy.negotiate(&unsupported),
        Err(ClientContractError::NoMutualVersion)
    ));
    assert!(ClientVersionOffer::new(Vec::new()).is_err());
    assert!(ClientVersionOffer::new((1..=17).map(version)).is_err());
    assert!(
        policy
            .decode_offer(br#"{"contract":"another","supported_versions":[1]}"#)
            .is_err()
    );
}

#[test]
fn typed_request_round_trip_rejects_schema_drift_and_missing_mutation_key() {
    let policy = ClientContractPolicy::default();
    let request = ClientRequest::new(
        CURRENT_CLIENT_CONTRACT_VERSION,
        request_id(),
        LessonAttempt {
            lesson_id: "lesson_1".to_string(),
            answer: "bonjour".to_string(),
        },
    );
    assert!(matches!(
        request.require_idempotency_key(),
        Err(ClientContractError::MissingIdempotencyKey)
    ));

    let encoded = policy.encode_request(&request).expect("encoded request");
    let decoded = policy
        .decode_request::<LessonAttempt>(&encoded)
        .expect("decoded request");
    assert_eq!(decoded, request);

    let mut value: serde_json::Value =
        serde_json::from_slice(&encoded).expect("request JSON value");
    value["authority"] = serde_json::json!("admin");
    let unknown = serde_json::to_vec(&value).expect("unknown-field fixture");
    assert!(matches!(
        policy.decode_request::<LessonAttempt>(&unknown),
        Err(ClientContractError::InvalidJson(_))
    ));
}

#[test]
fn mutation_and_server_response_preserve_correlation_without_client_authority() {
    let policy = ClientContractPolicy::default();
    let request = ClientRequest::mutation(
        CURRENT_CLIENT_CONTRACT_VERSION,
        request_id(),
        IdempotencyKey::new("attempt_01j8lesson").expect("valid replay key"),
        LessonAttempt {
            lesson_id: "lesson_1".to_string(),
            answer: "bonjour".to_string(),
        },
    );
    assert_eq!(
        request
            .require_idempotency_key()
            .expect("mutation replay key")
            .as_str(),
        "attempt_01j8lesson"
    );

    let response = ServerResponse::new(
        request.version(),
        request.request_id().clone(),
        1_782_000_000_000,
        LessonResult {
            correct: true,
            awarded_points: 10,
        },
    );
    let encoded = policy.encode_response(&response).expect("encoded response");
    let decoded = policy
        .decode_response::<LessonResult>(&encoded)
        .expect("decoded response");
    assert_eq!(decoded, response);
    assert_eq!(decoded.request_id(), request.request_id());
    assert_eq!(decoded.server_epoch_ms(), 1_782_000_000_000);

    let wire = String::from_utf8(encoded).expect("UTF-8 JSON response");
    assert!(!wire.contains("role"));
    assert!(!wire.contains("tenant"));
    assert!(!wire.contains("authorized"));
}

#[test]
fn failure_envelope_is_machine_readable_and_message_free() {
    let policy = ClientContractPolicy::default();
    let failure = ServerFailure::new(
        CURRENT_CLIENT_CONTRACT_VERSION,
        request_id(),
        1_782_000_000_000,
        FailureDetail::new(
            FailureCode::new("lesson.answer_invalid").expect("valid failure code"),
            false,
        ),
    );
    let encoded = policy.encode_failure(&failure).expect("encoded failure");
    let decoded = policy.decode_failure(&encoded).expect("decoded failure");
    assert_eq!(decoded, failure);
    assert_eq!(decoded.error().code().as_str(), "lesson.answer_invalid");
    assert!(!decoded.error().retryable());
    assert!(
        !String::from_utf8(encoded)
            .expect("UTF-8 JSON failure")
            .contains("message")
    );
}

#[test]
fn codec_enforces_version_and_size_before_application_processing() {
    let policy =
        ClientContractPolicy::new(version(1), version(1), 256).expect("valid bounded policy");
    let future_request = ClientRequest::new(
        version(2),
        request_id(),
        LessonAttempt {
            lesson_id: "lesson_1".to_string(),
            answer: "bonjour".to_string(),
        },
    );
    assert!(matches!(
        policy.encode_request(&future_request),
        Err(ClientContractError::UnsupportedVersion { received: 2, .. })
    ));

    let oversized = vec![b' '; 257];
    assert!(matches!(
        policy.decode_request::<LessonAttempt>(&oversized),
        Err(ClientContractError::BodyTooLarge { maximum: 256 })
    ));
    let oversized_payload = ClientRequest::new(
        version(1),
        request_id(),
        LessonAttempt {
            lesson_id: "lesson_1".to_string(),
            answer: "x".repeat(300),
        },
    );
    assert!(matches!(
        policy.encode_request(&oversized_payload),
        Err(ClientContractError::BodyTooLarge { maximum: 256 })
    ));
    assert!(ClientContractPolicy::new(version(2), version(1), 256).is_err());
    assert!(ClientContractPolicy::new(version(1), version(1), 0).is_err());
    assert!(
        ClientContractPolicy::new(version(1), version(1), MAX_CLIENT_CONTRACT_BODY_BYTES + 1)
            .is_err()
    );
}
