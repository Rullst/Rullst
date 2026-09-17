#![cfg(feature = "age-assurance")]

use ring::signature::{Ed25519KeyPair, KeyPair};
use rullst_privacy::age_assurance::*;
use std::sync::Arc;

fn key(seed: u8) -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(&[seed; 32]).unwrap()
}

fn issuer() -> TrustedIssuer {
    TrustedIssuer::new(
        "evaluated-issuer",
        "key-1",
        key(1).public_key().as_ref().try_into().unwrap(),
        [
            AgeMethod::SelfDeclaration,
            AgeMethod::FacialEstimation,
            AgeMethod::VerifiedAttribute,
        ],
    )
    .unwrap()
}

fn binding() -> SubjectBinding {
    SubjectBinding::new(
        "subject-opaque",
        "school-1",
        "session-ref",
        "academy",
        "restricted-action",
    )
    .unwrap()
}

fn policy() -> AgePolicy {
    AgePolicy::new("policy-v1", RiskLevel::Elevated, 18).unwrap()
}

fn challenge(method: AgeMethod) -> AgeChallenge {
    AgeChallenge::issue(&policy(), binding(), method, 1000).unwrap()
}

fn signed(challenge: &AgeChallenge, outcome: AgeOutcome) -> (Vec<u8>, Vec<u8>) {
    let payload = encode_attestation("evaluated-issuer", "key-1", challenge, outcome).unwrap();
    let signature = key(1)
        .sign(&signing_message(&payload).unwrap())
        .as_ref()
        .to_vec();
    (payload, signature)
}

fn verifier() -> AgeVerifier<MemoryReplayStore> {
    AgeVerifier::for_development(issuer(), MemoryReplayStore::new(100).unwrap())
}

#[test]
fn risk_policies_do_not_promote_weak_methods() {
    for (risk, accepted) in [
        (RiskLevel::Low, [true, true, true]),
        (RiskLevel::Elevated, [false, true, true]),
        (RiskLevel::Restricted, [false, false, true]),
    ] {
        let policy = AgePolicy::new("review-v1", risk, 16).unwrap();
        for (method, allowed) in [
            AgeMethod::SelfDeclaration,
            AgeMethod::FacialEstimation,
            AgeMethod::VerifiedAttribute,
        ]
        .into_iter()
        .zip(accepted)
        {
            assert_eq!(
                AgeChallenge::issue(&policy, binding(), method, 10).is_ok(),
                allowed
            );
        }
    }
}

#[test]
fn facial_threshold_includes_margin_but_failure_does_not_prove_minor_status() {
    let challenge = challenge(AgeMethod::FacialEstimation);
    assert_eq!(challenge.threshold(), 21);
    let (payload, signature) = signed(&challenge, AgeOutcome::BelowThreshold);
    let result = verifier()
        .verify(
            &policy(),
            &binding(),
            &challenge,
            &payload,
            &signature,
            1001,
        )
        .unwrap();
    assert_eq!(result.decision(), AgeDecision::AlternativeRequired);
    assert_eq!(result.assurance(), Assurance::Estimated);
}

#[test]
fn all_terminal_outcomes_are_explicit() {
    for (outcome, decision) in [
        (AgeOutcome::MeetsThreshold, AgeDecision::Allowed),
        (AgeOutcome::BelowThreshold, AgeDecision::BelowMinimumAge),
        (AgeOutcome::Inconclusive, AgeDecision::AlternativeRequired),
        (AgeOutcome::Unavailable, AgeDecision::Unavailable),
    ] {
        let challenge = challenge(AgeMethod::VerifiedAttribute);
        let (payload, signature) = signed(&challenge, outcome);
        let verifier = verifier();
        let result = verifier
            .verify(
                &policy(),
                &binding(),
                &challenge,
                &payload,
                &signature,
                1001,
            )
            .unwrap();
        assert_eq!(result.decision(), decision);
        assert_eq!(result.assurance(), Assurance::VerifiedAttribute);
        assert_eq!(result.policy_version(), "policy-v1");
        assert_eq!(result.expires_at(), 1300);
        assert_eq!(
            verifier.verify(
                &policy(),
                &binding(),
                &challenge,
                &payload,
                &signature,
                1001
            ),
            Err(AgeError::Replay)
        );
    }
}

#[test]
fn self_declaration_remains_labeled_declared() {
    let policy = AgePolicy::new("low-v1", RiskLevel::Low, 13).unwrap();
    let challenge =
        AgeChallenge::issue(&policy, binding(), AgeMethod::SelfDeclaration, 1000).unwrap();
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let result = verifier()
        .verify(&policy, &binding(), &challenge, &payload, &signature, 1001)
        .unwrap();
    assert_eq!(result.assurance(), Assurance::Declared);
}

#[test]
fn every_context_dimension_is_checked_before_consuming_evidence() {
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let verifier = verifier();
    for index in 0..5 {
        let mut values = [
            "subject-opaque",
            "school-1",
            "session-ref",
            "academy",
            "restricted-action",
        ];
        values[index] = "another-context";
        let other =
            SubjectBinding::new(values[0], values[1], values[2], values[3], values[4]).unwrap();
        assert_eq!(
            verifier.verify(&policy(), &other, &challenge, &payload, &signature, 1001),
            Err(AgeError::BindingMismatch)
        );
    }
    assert!(
        verifier
            .verify(
                &policy(),
                &binding(),
                &challenge,
                &payload,
                &signature,
                1001
            )
            .is_ok()
    );
}

#[test]
fn policy_changes_even_with_same_version_invalidate_old_challenges() {
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let changed = policy().with_estimation_margin(5).unwrap();
    assert_eq!(
        verifier().verify(&changed, &binding(), &challenge, &payload, &signature, 1001),
        Err(AgeError::BindingMismatch)
    );
}

#[test]
fn proof_is_bound_to_random_challenge_and_exact_signed_bytes() {
    let first = challenge(AgeMethod::VerifiedAttribute);
    let second = challenge(AgeMethod::VerifiedAttribute);
    assert_ne!(
        first.request_json().unwrap(),
        second.request_json().unwrap()
    );
    let (mut payload, signature) = signed(&first, AgeOutcome::MeetsThreshold);
    assert_eq!(
        verifier().verify(&policy(), &binding(), &second, &payload, &signature, 1001),
        Err(AgeError::BindingMismatch)
    );
    payload.push(b' ');
    assert_eq!(
        verifier().verify(&policy(), &binding(), &first, &payload, &signature, 1001),
        Err(AgeError::InvalidSignature)
    );
}

#[test]
fn clock_boundaries_have_no_expiry_grace() {
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    for (now, expected) in [
        (999, AgeError::InvalidChallenge),
        (1300, AgeError::Expired),
        (i64::MAX, AgeError::Expired),
    ] {
        assert_eq!(
            verifier().verify(&policy(), &binding(), &challenge, &payload, &signature, now),
            Err(expected)
        );
    }
    assert!(
        verifier()
            .verify(
                &policy(),
                &binding(),
                &challenge,
                &payload,
                &signature,
                1299
            )
            .is_ok()
    );
    assert!(
        AgeChallenge::issue(&policy(), binding(), AgeMethod::VerifiedAttribute, i64::MAX).is_err()
    );
    assert!(AgeChallenge::issue(&policy(), binding(), AgeMethod::VerifiedAttribute, -1).is_err());
}

#[test]
fn invalid_keys_signatures_and_unsigned_inputs_fail_closed() {
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let (payload, _) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let wrong = key(2).sign(&signing_message(&payload).unwrap());
    let bare = key(1).sign(&payload);
    for signature in [wrong.as_ref(), bare.as_ref(), &[], &[0; 64], &[0; 65]] {
        assert!(
            verifier()
                .verify(&policy(), &binding(), &challenge, &payload, signature, 1001)
                .is_err()
        );
    }
}

#[test]
fn strict_codec_denies_extra_data_unknown_versions_keys_and_oversized_input() {
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let (payload, _) = signed(&challenge, AgeOutcome::MeetsThreshold);
    for (field, value) in [
        ("version", serde_json::json!(2)),
        ("issuer", serde_json::json!("unknown")),
        ("key_id", serde_json::json!("unknown")),
        ("photo", serde_json::json!("unnecessary-personal-data")),
    ] {
        let mut body: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        body[field] = value;
        let modified = serde_json::to_vec(&body).unwrap();
        let signature = key(1).sign(&signing_message(&modified).unwrap());
        assert!(
            verifier()
                .verify(
                    &policy(),
                    &binding(),
                    &challenge,
                    &modified,
                    signature.as_ref(),
                    1001
                )
                .is_err()
        );
    }
    assert_eq!(
        signing_message(&vec![b' '; 4097]),
        Err(AgeError::InvalidAttestation)
    );
    assert_eq!(signing_message(&[]), Err(AgeError::InvalidAttestation));
}

#[test]
fn issuer_capabilities_and_key_rotation_are_explicit() {
    let limited = TrustedIssuer::new(
        "evaluated-issuer",
        "key-1",
        key(1).public_key().as_ref().try_into().unwrap(),
        [AgeMethod::VerifiedAttribute],
    )
    .unwrap();
    let facial = challenge(AgeMethod::FacialEstimation);
    let (payload, signature) = signed(&facial, AgeOutcome::MeetsThreshold);
    let limited_verifier =
        AgeVerifier::for_development(limited, MemoryReplayStore::new(10).unwrap());
    assert_eq!(
        limited_verifier.verify(&policy(), &binding(), &facial, &payload, &signature, 1001),
        Err(AgeError::MethodNotAllowed)
    );
    let rotated = issuer()
        .with_key("key-2", key(2).public_key().as_ref().try_into().unwrap())
        .unwrap();
    let verifier = AgeVerifier::for_development(rotated, MemoryReplayStore::new(10).unwrap());
    let proof = encode_attestation(
        "evaluated-issuer",
        "key-2",
        &facial,
        AgeOutcome::MeetsThreshold,
    )
    .unwrap();
    let signature = key(2).sign(&signing_message(&proof).unwrap());
    assert!(
        verifier
            .verify(
                &policy(),
                &binding(),
                &facial,
                &proof,
                signature.as_ref(),
                1001
            )
            .is_ok()
    );
    assert!(issuer().with_key("key-1", [1; 32]).is_err());
}

#[test]
fn invalid_configuration_cannot_create_a_usable_policy() {
    for age in [0, 121, 255] {
        assert!(AgePolicy::new("v1", RiskLevel::Low, age).is_err());
    }
    for id in ["", "contains spaces", "contains/email@address"] {
        assert!(AgePolicy::new(id, RiskLevel::Low, 18).is_err());
    }
    assert!(policy().with_estimation_margin(0).is_err());
    assert!(policy().with_validity_seconds(0).is_err());
    assert!(policy().with_validity_seconds(901).is_err());
    let high_age = AgePolicy::new("v1", RiskLevel::Low, 120).unwrap();
    assert!(AgeChallenge::issue(&high_age, binding(), AgeMethod::FacialEstimation, 10).is_err());
    // Deserialization is not an escape from constructor invariants.
    let mut wire = serde_json::to_value(policy()).unwrap();
    wire["minimum_age"] = serde_json::json!(0);
    let malformed: AgePolicy = serde_json::from_value(wire).unwrap();
    assert!(AgeChallenge::issue(&malformed, binding(), AgeMethod::VerifiedAttribute, 10).is_err());
}

#[test]
fn local_store_never_evicts_unexpired_claims_to_make_room() {
    let store = MemoryReplayStore::new(1).unwrap();
    assert!(store.claim([1; 32], 20, 10).unwrap());
    assert_eq!(store.claim([2; 32], 30, 10), Err(AgeError::StoreCapacity));
    assert!(!store.claim([1; 32], 20, 10).unwrap());
    assert!(store.claim([2; 32], 30, 20).unwrap());
    assert!(MemoryReplayStore::new(0).is_err());
    assert!(MemoryReplayStore::new(100_001).is_err());
}

#[test]
fn concurrent_verifiers_consume_a_proof_once() {
    let store = Arc::new(MemoryReplayStore::new(10).unwrap());
    let challenge = Arc::new(challenge(AgeMethod::VerifiedAttribute));
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let verifier = AgeVerifier::for_development(issuer(), store.clone());
            let (challenge, payload, signature) =
                (challenge.clone(), payload.clone(), signature.clone());
            std::thread::spawn(move || {
                verifier.verify(
                    &policy(),
                    &binding(),
                    &challenge,
                    &payload,
                    &signature,
                    1001,
                )
            })
        })
        .collect();
    let mut allowed = 0;
    for thread in threads {
        match thread.join().unwrap() {
            Ok(result) => {
                assert_eq!(result.decision(), AgeDecision::Allowed);
                allowed += 1;
            }
            Err(error) => assert_eq!(error, AgeError::Replay),
        }
    }
    assert_eq!(allowed, 1);
}

// A failure fixture, never evidence of a real shared durable backend.
struct UnavailableDurableStore;
impl ReplayStore for UnavailableDurableStore {
    fn durability(&self) -> ReplayDurability {
        ReplayDurability::SharedDurable
    }
    fn claim(&self, _: [u8; 32], _: i64, _: i64) -> Result<bool, AgeError> {
        Err(AgeError::StoreUnavailable)
    }
}

#[test]
fn production_denies_local_state_mocks_and_failed_durable_claims() {
    assert!(matches!(
        AgeVerifier::new(issuer(), MemoryReplayStore::new(1).unwrap()),
        Err(AgeError::DurableReplayRequired)
    ));
    let verifier = AgeVerifier::new(issuer(), UnavailableDurableStore).unwrap();
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let mock = MockAgeProvider::new("", AgeOutcome::MeetsThreshold).unwrap();
    assert_eq!(
        verifier.verify_mock(&policy(), &binding(), &challenge, &mock, 1001),
        Err(AgeError::MockInProduction)
    );
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    assert_eq!(
        verifier.verify(
            &policy(),
            &binding(),
            &challenge,
            &payload,
            &signature,
            1001
        ),
        Err(AgeError::StoreUnavailable)
    );
}

#[test]
fn mocks_are_explicit_and_debug_output_omits_subject_references() {
    assert!(MockAgeProvider::new("live-credential", AgeOutcome::MeetsThreshold).is_err());
    for credentials in ["", "mock_local"] {
        let mock = MockAgeProvider::new(credentials, AgeOutcome::MeetsThreshold).unwrap();
        let challenge = challenge(AgeMethod::VerifiedAttribute);
        let result = verifier()
            .verify_mock(&policy(), &binding(), &challenge, &mock, 1001)
            .unwrap();
        assert_eq!(result.assurance(), Assurance::OfflineMock);
        for debug in [
            format!("{:?}", binding()),
            format!("{challenge:?}"),
            format!("{result:?}"),
        ] {
            assert!(!debug.contains("subject-opaque"));
            assert!(!debug.contains("session-ref"));
        }
    }
}
