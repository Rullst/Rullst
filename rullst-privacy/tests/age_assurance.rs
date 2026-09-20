#![cfg(feature = "age-assurance")]

use ring::signature::{Ed25519KeyPair, KeyPair};
use rullst_privacy::age_assurance::*;
use std::sync::Arc;

struct FixedClock(i64);
impl AgeClock for FixedClock {
    fn now(&self) -> Result<i64, AgeError> {
        Ok(self.0)
    }
}

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

#[path = "age_assurance/clock.rs"]
mod clock;
#[path = "age_assurance/contracts.rs"]
mod contracts;
#[cfg(feature = "postgres")]
#[path = "age_assurance/postgres/mod.rs"]
mod postgres;
#[path = "age_assurance/replay.rs"]
mod replay;
#[cfg(feature = "sqlite")]
#[path = "age_assurance/sqlite.rs"]
mod sqlite;

async fn assess<S: ReplayStore>(
    verifier: &AgeVerifier<S>,
    challenge: &AgeChallenge,
    clock: &impl AgeClock,
) -> Result<AgeAssessment, AgeError> {
    let (payload, signature) = signed(challenge, AgeOutcome::MeetsThreshold);
    verifier
        .verify_with_clock(
            &policy(),
            &binding(),
            challenge,
            &payload,
            &signature,
            clock,
        )
        .await
}
