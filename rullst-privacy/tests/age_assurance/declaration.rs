use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

fn low_policy() -> AgePolicy {
    AgePolicy::new("self-declaration-v1", RiskLevel::Low, 18).unwrap()
}

fn gate() -> DeclarationGate<MemoryReplayStore> {
    DeclarationGate::for_development(MemoryReplayStore::new(10).unwrap())
}

fn issued(policy: &AgePolicy, method: AgeMethod) -> AgeChallenge {
    AgeChallenge::issue(policy, binding(), method, 1000).unwrap()
}

#[tokio::test]
async fn explicit_answers_never_become_verified_age_and_all_are_single_use() {
    let policy = low_policy();
    for (answer, expected) in [
        (AgeDeclaration::MeetsThreshold, AgeDecision::Allowed),
        (AgeDeclaration::BelowThreshold, AgeDecision::BelowMinimumAge),
        (AgeDeclaration::Declined, AgeDecision::AlternativeRequired),
    ] {
        let gate = gate();
        let challenge = issued(&policy, AgeMethod::SelfDeclaration);
        let assessment = gate
            .assess_with_clock(&policy, &binding(), &challenge, answer, &FixedClock(1001))
            .await
            .unwrap();
        assert_eq!(assessment.decision(), expected);
        assert_eq!(assessment.assurance(), Assurance::Declared);
        assert_eq!(assessment.policy_version(), policy.version());
        assert_eq!(assessment.expires_at(), challenge.expires_at());
        assert_eq!(
            gate.assess_with_clock(
                &policy,
                &binding(),
                &challenge,
                AgeDeclaration::MeetsThreshold,
                &FixedClock(1001)
            )
            .await,
            Err(AgeError::Replay)
        );
    }
    for input in ["null", "true", "false", "{}", "\"yes\"", "\"verified\""] {
        assert!(serde_json::from_str::<AgeDeclaration>(input).is_err());
    }
}

#[tokio::test]
async fn stronger_methods_and_changed_policies_cannot_use_the_declaration_path() {
    for risk in [RiskLevel::Low, RiskLevel::Elevated, RiskLevel::Restricted] {
        let policy = AgePolicy::new("current-policy", risk, 18).unwrap();
        for method in [AgeMethod::FacialEstimation, AgeMethod::VerifiedAttribute] {
            if !policy.permits(method) {
                continue;
            }
            let challenge = issued(&policy, method);
            assert_eq!(
                gate()
                    .assess_with_clock(
                        &policy,
                        &binding(),
                        &challenge,
                        AgeDeclaration::MeetsThreshold,
                        &FixedClock(1001)
                    )
                    .await,
                Err(AgeError::MethodNotAllowed)
            );
        }
    }
    let policy = low_policy();
    let challenge = issued(&policy, AgeMethod::SelfDeclaration);
    for changed in [
        AgePolicy::new(policy.version(), RiskLevel::Elevated, 18).unwrap(),
        AgePolicy::new(policy.version(), RiskLevel::Low, 21).unwrap(),
        AgePolicy::new("next-version", RiskLevel::Low, 18).unwrap(),
    ] {
        assert_eq!(
            gate()
                .assess_with_clock(
                    &changed,
                    &binding(),
                    &challenge,
                    AgeDeclaration::MeetsThreshold,
                    &FixedClock(1001)
                )
                .await,
            Err(AgeError::BindingMismatch)
        );
    }
}

#[tokio::test]
async fn current_authenticated_context_must_match_before_consumption() {
    let policy = low_policy();
    let challenge = issued(&policy, AgeMethod::SelfDeclaration);
    let gate = gate();
    let references = [
        "subject-opaque",
        "school-1",
        "session-ref",
        "academy",
        "restricted-action",
    ];
    for index in 0..references.len() {
        let mut changed = references;
        changed[index] = "another-context";
        let context =
            SubjectBinding::new(changed[0], changed[1], changed[2], changed[3], changed[4])
                .unwrap();
        assert_eq!(
            gate.assess_with_clock(
                &policy,
                &context,
                &challenge,
                AgeDeclaration::MeetsThreshold,
                &FixedClock(1001)
            )
            .await,
            Err(AgeError::BindingMismatch)
        );
    }
    assert_eq!(
        gate.assess_with_clock(
            &policy,
            &binding(),
            &challenge,
            AgeDeclaration::MeetsThreshold,
            &FixedClock(1001)
        )
        .await
        .unwrap()
        .decision(),
        AgeDecision::Allowed
    );
}

struct ChangingClock {
    calls: AtomicUsize,
    after: i64,
}
impl AgeClock for ChangingClock {
    fn now(&self) -> Result<i64, AgeError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(1001)
        } else {
            Ok(self.after)
        }
    }
}

#[tokio::test]
async fn time_is_rechecked_after_native_declaration_consumption() {
    let policy = low_policy();
    for (after, expected) in [(1300, AgeError::Expired), (1000, AgeError::ClockRollback)] {
        let gate = gate();
        let challenge = issued(&policy, AgeMethod::SelfDeclaration);
        let clock = ChangingClock {
            calls: AtomicUsize::new(0),
            after,
        };
        assert_eq!(
            gate.assess_with_clock(
                &policy,
                &binding(),
                &challenge,
                AgeDeclaration::MeetsThreshold,
                &clock
            )
            .await,
            Err(expected)
        );
        assert_eq!(
            gate.assess_with_clock(
                &policy,
                &binding(),
                &challenge,
                AgeDeclaration::MeetsThreshold,
                &FixedClock(1001)
            )
            .await,
            Err(AgeError::Replay)
        );
    }
    let challenge = AgeChallenge::issue(
        &policy,
        binding(),
        AgeMethod::SelfDeclaration,
        SystemAgeClock.now().unwrap(),
    )
    .unwrap();
    assert_eq!(
        gate()
            .assess(
                &policy,
                &binding(),
                &challenge,
                AgeDeclaration::MeetsThreshold
            )
            .await
            .unwrap()
            .decision(),
        AgeDecision::Allowed
    );
}

// Test-only failure/mode-change fixture; not evidence of a durable backend.
struct ChangingStore {
    shared: Arc<AtomicBool>,
    claims: Arc<AtomicUsize>,
}
impl ReplayStore for ChangingStore {
    fn durability(&self) -> ReplayDurability {
        if self.shared.load(Ordering::SeqCst) {
            ReplayDurability::SharedDurable
        } else {
            ReplayDurability::ProcessLocal
        }
    }
    async fn claim(&self, _: [u8; 32], _: i64, _: i64) -> Result<bool, AgeError> {
        self.claims.fetch_add(1, Ordering::SeqCst);
        Err(AgeError::StoreUnavailable)
    }
}

#[tokio::test]
async fn production_denies_local_downgraded_and_unavailable_replay_state() {
    assert!(matches!(
        DeclarationGate::new(MemoryReplayStore::new(10).unwrap()),
        Err(AgeError::DurableReplayRequired)
    ));
    let shared = Arc::new(AtomicBool::new(true));
    let claims = Arc::new(AtomicUsize::new(0));
    let gate = DeclarationGate::new(ChangingStore {
        shared: shared.clone(),
        claims: claims.clone(),
    })
    .unwrap();
    let policy = low_policy();
    let challenge = issued(&policy, AgeMethod::SelfDeclaration);
    shared.store(false, Ordering::SeqCst);
    assert_eq!(
        gate.assess_with_clock(
            &policy,
            &binding(),
            &challenge,
            AgeDeclaration::MeetsThreshold,
            &FixedClock(1001)
        )
        .await,
        Err(AgeError::DurableReplayRequired)
    );
    assert_eq!(claims.load(Ordering::SeqCst), 0);
    shared.store(true, Ordering::SeqCst);
    assert_eq!(
        gate.assess_with_clock(
            &policy,
            &binding(),
            &challenge,
            AgeDeclaration::MeetsThreshold,
            &FixedClock(1001)
        )
        .await,
        Err(AgeError::StoreUnavailable)
    );
    assert_eq!(claims.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn first_party_and_signed_declarations_share_the_same_replay_boundary() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("declarations.sqlite");
    let first = SqliteReplayStore::open(&path, 10).await.unwrap();
    let second = SqliteReplayStore::open(&path, 10).await.unwrap();
    let native = DeclarationGate::new(first.clone()).unwrap();
    let signed_verifier = AgeVerifier::new(issuer(), second.clone()).unwrap();
    let policy = low_policy();
    let challenge = issued(&policy, AgeMethod::SelfDeclaration);
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let context = binding();
    let (a, b) = tokio::join!(
        native.assess_with_clock(
            &policy,
            &context,
            &challenge,
            AgeDeclaration::MeetsThreshold,
            &FixedClock(1001)
        ),
        signed_verifier.verify_with_clock(
            &policy,
            &context,
            &challenge,
            &payload,
            &signature,
            &FixedClock(1001)
        )
    );
    assert!(matches!(
        (&a, &b),
        (Ok(_), Err(AgeError::Replay)) | (Err(AgeError::Replay), Ok(_))
    ));
    let assessment = a.or(b).unwrap();
    assert_eq!(assessment.decision(), AgeDecision::Allowed);
    assert_eq!(assessment.assurance(), Assurance::Declared);
    first.close().await;
    second.close().await;
}
