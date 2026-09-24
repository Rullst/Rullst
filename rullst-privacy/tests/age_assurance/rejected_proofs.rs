use super::*;

async fn rejected_signature_does_not_spend_the_challenge<S: ReplayStore>(verifier: AgeVerifier<S>) {
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    let mut corrupt_signature = signature.clone();
    corrupt_signature[0] ^= 1;
    for rejected_signature in [&corrupt_signature[..], &signature[..63]] {
        assert_eq!(
            verifier
                .verify_with_clock(
                    &policy(),
                    &binding(),
                    &challenge,
                    &payload,
                    rejected_signature,
                    &FixedClock(1001),
                )
                .await,
            Err(AgeError::InvalidSignature),
        );
    }
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001))
            .await
            .unwrap()
            .decision(),
        AgeDecision::Allowed
    );
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001)).await,
        Err(AgeError::Replay)
    );
    let another = self::challenge(AgeMethod::VerifiedAttribute);
    assert_eq!(
        assess(&verifier, &another, &FixedClock(1001)).await,
        Err(AgeError::StoreCapacity)
    );
}

#[tokio::test]
async fn invalid_signature_does_not_poison_the_memory_replay_slot() {
    rejected_signature_does_not_spend_the_challenge(AgeVerifier::for_development(
        issuer(),
        MemoryReplayStore::new(1).unwrap(),
    ))
    .await;
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn invalid_signature_does_not_poison_the_durable_replay_slot() {
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteReplayStore::open(directory.path().join("rejected.sqlite"), 1)
        .await
        .unwrap();
    rejected_signature_does_not_spend_the_challenge(
        AgeVerifier::new(issuer(), store.clone()).unwrap(),
    )
    .await;
    store.close().await;
}
