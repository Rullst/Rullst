use super::*;

#[tokio::test]
async fn risk_policies_do_not_promote_weak_methods() {
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

#[tokio::test]
async fn facial_threshold_includes_margin_but_failure_does_not_prove_minor_status() {
    let challenge = challenge(AgeMethod::FacialEstimation);
    assert_eq!(challenge.threshold(), 21);
    let (payload, signature) = signed(&challenge, AgeOutcome::BelowThreshold);
    let result = verifier()
        .verify_with_clock(
            &policy(),
            &binding(),
            &challenge,
            &payload,
            &signature,
            &FixedClock(1001),
        )
        .await
        .unwrap();
    assert_eq!(result.decision(), AgeDecision::AlternativeRequired);
    assert_eq!(result.assurance(), Assurance::Estimated);
}

#[tokio::test]
async fn all_terminal_outcomes_are_explicit() {
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
            .verify_with_clock(
                &policy(),
                &binding(),
                &challenge,
                &payload,
                &signature,
                &FixedClock(1001),
            )
            .await
            .unwrap();
        assert_eq!(result.decision(), decision);
        assert_eq!(result.assurance(), Assurance::VerifiedAttribute);
        assert_eq!(result.policy_version(), "policy-v1");
        assert_eq!(result.expires_at(), 1300);
        assert_eq!(
            verifier
                .verify_with_clock(
                    &policy(),
                    &binding(),
                    &challenge,
                    &payload,
                    &signature,
                    &FixedClock(1001)
                )
                .await,
            Err(AgeError::Replay)
        );
    }
}

#[tokio::test]
async fn self_declaration_remains_labeled_declared() {
    let policy = AgePolicy::new("low-v1", RiskLevel::Low, 13).unwrap();
    let challenge =
        AgeChallenge::issue(&policy, binding(), AgeMethod::SelfDeclaration, 1000).unwrap();
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let result = verifier()
        .verify_with_clock(
            &policy,
            &binding(),
            &challenge,
            &payload,
            &signature,
            &FixedClock(1001),
        )
        .await
        .unwrap();
    assert_eq!(result.assurance(), Assurance::Declared);
}

#[tokio::test]
async fn every_context_dimension_is_checked_before_consuming_evidence() {
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
            verifier
                .verify_with_clock(
                    &policy(),
                    &other,
                    &challenge,
                    &payload,
                    &signature,
                    &FixedClock(1001)
                )
                .await,
            Err(AgeError::BindingMismatch)
        );
    }
    assert!(
        verifier
            .verify_with_clock(
                &policy(),
                &binding(),
                &challenge,
                &payload,
                &signature,
                &FixedClock(1001)
            )
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn policy_changes_even_with_same_version_invalidate_old_challenges() {
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let changed = policy().with_estimation_margin(5).unwrap();
    assert_eq!(
        verifier()
            .verify_with_clock(
                &changed,
                &binding(),
                &challenge,
                &payload,
                &signature,
                &FixedClock(1001)
            )
            .await,
        Err(AgeError::BindingMismatch)
    );
}

#[tokio::test]
async fn proof_is_bound_to_random_challenge_and_exact_signed_bytes() {
    let first = challenge(AgeMethod::VerifiedAttribute);
    let second = challenge(AgeMethod::VerifiedAttribute);
    assert_ne!(
        first.request_json().unwrap(),
        second.request_json().unwrap()
    );
    let (mut payload, signature) = signed(&first, AgeOutcome::MeetsThreshold);
    assert_eq!(
        verifier()
            .verify_with_clock(
                &policy(),
                &binding(),
                &second,
                &payload,
                &signature,
                &FixedClock(1001)
            )
            .await,
        Err(AgeError::BindingMismatch)
    );
    payload.push(b' ');
    assert_eq!(
        verifier()
            .verify_with_clock(
                &policy(),
                &binding(),
                &first,
                &payload,
                &signature,
                &FixedClock(1001)
            )
            .await,
        Err(AgeError::InvalidSignature)
    );
}

#[tokio::test]
async fn clock_boundaries_have_no_expiry_grace() {
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    for (now, expected) in [
        (999, AgeError::InvalidChallenge),
        (1300, AgeError::Expired),
        (i64::MAX, AgeError::Expired),
    ] {
        assert_eq!(
            verifier()
                .verify_with_clock(
                    &policy(),
                    &binding(),
                    &challenge,
                    &payload,
                    &signature,
                    &FixedClock(now)
                )
                .await,
            Err(expected)
        );
    }
    assert!(
        verifier()
            .verify_with_clock(
                &policy(),
                &binding(),
                &challenge,
                &payload,
                &signature,
                &FixedClock(1299)
            )
            .await
            .is_ok()
    );
    assert!(
        AgeChallenge::issue(&policy(), binding(), AgeMethod::VerifiedAttribute, i64::MAX).is_err()
    );
    assert!(AgeChallenge::issue(&policy(), binding(), AgeMethod::VerifiedAttribute, -1).is_err());
}

#[tokio::test]
async fn invalid_keys_signatures_and_unsigned_inputs_fail_closed() {
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let (payload, _) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let wrong = key(2).sign(&signing_message(&payload).unwrap());
    let bare = key(1).sign(&payload);
    for signature in [wrong.as_ref(), bare.as_ref(), &[], &[0; 64], &[0; 65]] {
        assert!(
            verifier()
                .verify_with_clock(
                    &policy(),
                    &binding(),
                    &challenge,
                    &payload,
                    signature,
                    &FixedClock(1001)
                )
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn strict_codec_denies_extra_data_unknown_versions_keys_and_oversized_input() {
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
                .verify_with_clock(
                    &policy(),
                    &binding(),
                    &challenge,
                    &modified,
                    signature.as_ref(),
                    &FixedClock(1001)
                )
                .await
                .is_err()
        );
    }
    assert_eq!(
        signing_message(&vec![b' '; 4097]),
        Err(AgeError::InvalidAttestation)
    );
    assert_eq!(signing_message(&[]), Err(AgeError::InvalidAttestation));
}

#[tokio::test]
async fn issuer_capabilities_and_key_rotation_are_explicit() {
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
        limited_verifier
            .verify_with_clock(
                &policy(),
                &binding(),
                &facial,
                &payload,
                &signature,
                &FixedClock(1001)
            )
            .await,
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
            .verify_with_clock(
                &policy(),
                &binding(),
                &facial,
                &proof,
                signature.as_ref(),
                &FixedClock(1001)
            )
            .await
            .is_ok()
    );
    assert!(issuer().with_key("key-1", [1; 32]).is_err());
}

#[test]
fn small_order_and_non_canonical_issuer_keys_are_rejected() {
    fn encoding(low: u8, fill: u8, high: u8) -> [u8; 32] {
        let mut bytes = [fill; 32];
        bytes[0] = low;
        bytes[31] = high;
        bytes
    }
    let order_eight = [
        "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
        "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
    ]
    .map(|hex| {
        let mut bytes = [0u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap();
        }
        bytes
    });
    let mut degenerate = vec![
        [0; 32],
        encoding(1, 0, 0),          // identity: [h]A vanishes for every message
        encoding(0xec, 0xff, 0x7f), // y = p - 1, order 2
        encoding(0xed, 0xff, 0x7f), // y = p, a non-canonical 0
        encoding(0xee, 0xff, 0x7f), // y = p + 1, a non-canonical 1
        encoding(0xff, 0xff, 0x7f), // largest non-canonical y
    ];
    degenerate.extend(order_eight);
    for key in degenerate.clone() {
        let mut negative = key;
        negative[31] |= 0x80;
        degenerate.push(negative);
    }
    for public_key in degenerate {
        assert_eq!(
            TrustedIssuer::new(
                "evaluated-issuer",
                "key-1",
                public_key,
                [AgeMethod::VerifiedAttribute]
            )
            .err(),
            Some(AgeError::InvalidConfiguration)
        );
        assert!(issuer().with_key("key-2", public_key).is_err());
    }
    // Ordinary keys, including ones with the x sign bit set, remain accepted.
    for seed in 2..40 {
        let public_key: [u8; 32] = key(seed).public_key().as_ref().try_into().unwrap();
        assert!(issuer().with_key("key-2", public_key).is_ok());
    }
}

#[tokio::test]
async fn invalid_configuration_cannot_create_a_usable_policy() {
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
