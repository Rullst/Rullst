#![no_main]

use libfuzzer_sys::fuzz_target;
use ring::signature::{Ed25519KeyPair, KeyPair};
use rullst_privacy::age_assurance::*;
use rullst_privacy_fuzz::*;

fuzz_target!(|data: &[u8]| {
    if data.len() > 8192 {
        return;
    }
    let selector = data.first().copied().unwrap_or(0);
    let method = [
        AgeMethod::SelfDeclaration,
        AgeMethod::FacialEstimation,
        AgeMethod::VerifiedAttribute,
    ][usize::from(selector % 3)];
    let retained = challenge(method);
    let outcomes = [
        AgeOutcome::MeetsThreshold,
        AgeOutcome::BelowThreshold,
        AgeOutcome::Inconclusive,
        AgeOutcome::Unavailable,
    ];
    let original = encode_attestation(
        "fuzz-issuer",
        "key",
        &retained,
        outcomes[usize::from(selector / 3 % 4)],
    )
    .unwrap();
    let payload = match selector / 12 % 3 {
        0 => original,
        1 => data.get(1..).unwrap_or_default().to_vec(),
        _ => splice(&original, data.get(1..).unwrap_or_default()),
    };
    // Public deterministic fuzz-only key; signing mutations reaches semantic
    // validation of a trusted issuer's malformed response, not just bad signatures.
    let key = Ed25519KeyPair::from_seed_unchecked(&[8; 32]).unwrap();
    let mut signature = signing_message(&payload)
        .map(|message| key.sign(&message).as_ref().to_vec())
        .unwrap_or_default();
    let intact = selector & 128 == 0;
    if !intact && let Some(byte) = signature.first_mut() {
        *byte ^= 1;
    }
    let issuer = TrustedIssuer::new(
        "fuzz-issuer",
        "key",
        key.public_key().as_ref().try_into().unwrap(),
        [method],
    )
    .unwrap();
    let store = std::sync::Arc::new(MemoryReplayStore::new(2).unwrap());
    let verifier = AgeVerifier::for_development(issuer, store.clone());
    let other =
        SubjectBinding::new("subject", "other-tenant", "session", "audience", "action").unwrap();
    assert!(
        ready(verifier.verify_with_clock(
            &policy(),
            &other,
            &retained,
            &payload,
            &signature,
            &FixedClock(1001)
        ))
        .is_err()
    );
    let result = ready(verifier.verify_with_clock(
        &policy(),
        &binding(),
        &retained,
        &payload,
        &signature,
        &FixedClock(1001),
    ));
    if let Ok(assessment) = result {
        assert!(intact);
        let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(
            value["challenge"],
            serde_json::from_slice::<serde_json::Value>(&retained.request_json().unwrap()).unwrap()
        );
        let expected = match value["outcome"].as_str().unwrap() {
            "meets_threshold" => AgeDecision::Allowed,
            "below_threshold" if method != AgeMethod::FacialEstimation => {
                AgeDecision::BelowMinimumAge
            }
            "below_threshold" | "inconclusive" => AgeDecision::AlternativeRequired,
            "unavailable" => AgeDecision::Unavailable,
            _ => panic!("unrecognized accepted outcome"),
        };
        assert_eq!(assessment.decision(), expected);
        assert_eq!(
            assessment.assurance(),
            match method {
                AgeMethod::SelfDeclaration => Assurance::Declared,
                AgeMethod::FacialEstimation => Assurance::Estimated,
                AgeMethod::VerifiedAttribute => Assurance::VerifiedAttribute,
            }
        );
        assert_eq!(assessment.policy_version(), "fuzz-policy-v1");
        assert_eq!(assessment.expires_at(), 1300);
        assert!(matches!(
            ready(verifier.verify_with_clock(
                &policy(),
                &binding(),
                &retained,
                &payload,
                &signature,
                &FixedClock(1001)
            )),
            Err(AgeError::Replay)
        ));
        if method == AgeMethod::SelfDeclaration {
            let gate = DeclarationGate::for_development(store);
            assert!(matches!(
                ready(gate.assess_with_clock(
                    &policy(),
                    &binding(),
                    &retained,
                    AgeDeclaration::MeetsThreshold,
                    &FixedClock(1001)
                )),
                Err(AgeError::Replay)
            ));
        }
    }
});
