use super::*;

#[tokio::test]
async fn local_store_never_evicts_unexpired_claims_to_make_room() {
    let store = MemoryReplayStore::new(1).unwrap();
    assert!(store.claim([1; 32], 20, 10).await.unwrap());
    assert_eq!(
        store.claim([2; 32], 30, 10).await,
        Err(AgeError::StoreCapacity)
    );
    assert!(!store.claim([1; 32], 20, 10).await.unwrap());
    assert!(store.claim([2; 32], 30, 20).await.unwrap());
    assert!(MemoryReplayStore::new(0).is_err());
    assert!(MemoryReplayStore::new(100_001).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_verifiers_consume_a_proof_once() {
    let store = Arc::new(MemoryReplayStore::new(10).unwrap());
    let challenge = Arc::new(challenge(AgeMethod::VerifiedAttribute));
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let barrier = Arc::new(tokio::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let verifier = AgeVerifier::for_development(issuer(), store.clone());
            let (challenge, payload, signature) =
                (challenge.clone(), payload.clone(), signature.clone());
            let barrier = barrier.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                verifier
                    .verify_with_clock(
                        &policy(),
                        &binding(),
                        &challenge,
                        &payload,
                        &signature,
                        &FixedClock(1001),
                    )
                    .await
            })
        })
        .collect();
    let mut allowed = 0;
    for thread in threads {
        match thread.await.unwrap() {
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
    async fn claim(&self, _: [u8; 32], _: i64, _: i64) -> Result<bool, AgeError> {
        Err(AgeError::StoreUnavailable)
    }
}

#[tokio::test]
async fn production_denies_local_state_mocks_and_failed_durable_claims() {
    assert!(matches!(
        AgeVerifier::new(issuer(), MemoryReplayStore::new(1).unwrap()),
        Err(AgeError::DurableReplayRequired)
    ));
    let verifier = AgeVerifier::new(issuer(), UnavailableDurableStore).unwrap();
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let mock = MockAgeProvider::new("", AgeOutcome::MeetsThreshold).unwrap();
    assert_eq!(
        verifier
            .verify_mock_with_clock(&policy(), &binding(), &challenge, &mock, &FixedClock(1001))
            .await,
        Err(AgeError::MockInProduction)
    );
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
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
        Err(AgeError::StoreUnavailable)
    );
}

#[tokio::test]
async fn mocks_are_explicit_and_debug_output_omits_subject_references() {
    assert!(MockAgeProvider::new("live-credential", AgeOutcome::MeetsThreshold).is_err());
    for credentials in ["", "mock_local"] {
        let mock = MockAgeProvider::new(credentials, AgeOutcome::MeetsThreshold).unwrap();
        let challenge = challenge(AgeMethod::VerifiedAttribute);
        let result = verifier()
            .verify_mock_with_clock(&policy(), &binding(), &challenge, &mock, &FixedClock(1001))
            .await
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
