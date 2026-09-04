use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{sync::Arc, thread, time::Duration};

const KEY: &[u8] = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn observation(
    requests: u64,
    failures: u64,
    accounts: u64,
    paths: u64,
    sources: u64,
    correlated: u64,
) -> SentinelObservation {
    SentinelObservation::try_new(
        Duration::from_secs(60),
        requests,
        failures,
        accounts,
        paths,
        sources,
        correlated,
    )
    .expect("valid observation")
}

fn test_gate(capacity: usize) -> ProofOfWorkGate {
    ProofOfWorkGate::try_new(
        KEY,
        ProofOfWorkConfig::try_new(8, Duration::from_secs(30), capacity).expect("PoW config"),
    )
    .expect("PoW gate")
}

fn solve(challenge: &ProofOfWorkChallenge) -> u64 {
    (0..1_000_000)
        .find(|nonce| challenge.is_solution(*nonce))
        .expect("eight-bit challenge has a bounded solution")
}

fn find_non_solution(challenge: &ProofOfWorkChallenge) -> u64 {
    (0..1_000_000)
        .find(|nonce| !challenge.is_solution(*nonce))
        .expect("eight-bit challenge has a bounded non-solution")
}

#[test]
fn classifier_names_only_threshold_backed_patterns() {
    let classifier = ThreatClassifier::default();
    let credential = classifier.assess(observation(25, 20, 8, 3, 1, 0));
    assert_eq!(credential.patterns(), &[ThreatPattern::CredentialStuffing]);
    assert_eq!(credential.action(), SentinelAction::ProofOfWork);

    let scraping = classifier.assess(observation(400, 0, 0, 80, 1, 0));
    assert_eq!(scraping.patterns(), &[ThreatPattern::ApiScraping]);

    let distributed = classifier.assess(observation(500, 0, 0, 5, 40, 25));
    assert_eq!(
        distributed.patterns(),
        &[ThreatPattern::DistributedAutomation]
    );
    assert_eq!(distributed.risk_score(), 80);

    let normal = classifier.assess(observation(30, 1, 1, 4, 2, 0));
    assert!(normal.patterns().is_empty());
    assert_eq!(normal.risk_score(), 0);
    assert_eq!(normal.action(), SentinelAction::Observe);
}

#[test]
fn observations_and_policies_reject_inconsistent_or_unbounded_inputs() {
    assert!(SentinelObservation::try_new(Duration::ZERO, 1, 0, 0, 1, 1, 0).is_err());
    assert!(SentinelObservation::try_new(Duration::from_secs(60), 1, 2, 0, 1, 1, 0).is_err());
    assert!(SentinelObservation::try_new(Duration::from_secs(60), 1, 0, 0, 1, 1, 2).is_err());
    assert!(
        SentinelPolicy::default()
            .try_with_credential_stuffing(1, 1, 10_001)
            .is_err()
    );
    assert!(
        SentinelPolicy::default()
            .try_with_distributed_automation(1, 2, 3)
            .is_err()
    );
}

// TM-SEC-07: challenges are authenticated, subject-bound, expiring and exactly
// one concurrent verification consumes process-local state.
#[test]
fn proof_of_work_is_subject_bound_tamper_evident_expiring_and_one_shot() {
    let gate = test_gate(2);
    let challenge = gate
        .issue_at("peer:198.51.100.4".to_string(), 1_000)
        .expect("issued challenge");
    let solution = solve(&challenge);
    let invalid_solution = find_non_solution(&challenge);
    assert_eq!(gate.active_challenges(), 1);

    assert_eq!(
        gate.verify_at("peer:198.51.100.5", challenge.token(), solution, 1_001,),
        Err(SentinelError::InvalidToken)
    );
    assert_eq!(
        gate.verify_at(
            "peer:198.51.100.4",
            challenge.token(),
            invalid_solution,
            1_001,
        ),
        Err(SentinelError::InvalidProof)
    );

    let mut tampered = URL_SAFE_NO_PAD
        .decode(challenge.token())
        .expect("challenge encoding");
    tampered[1] ^= 1;
    assert_eq!(
        gate.verify_at(
            "peer:198.51.100.4",
            &URL_SAFE_NO_PAD.encode(tampered),
            solution,
            1_001,
        ),
        Err(SentinelError::InvalidToken)
    );

    let gate = Arc::new(gate);
    let mut workers = Vec::new();
    for _ in 0..16 {
        let gate = gate.clone();
        let token = challenge.token().to_string();
        workers.push(thread::spawn(move || {
            gate.verify_at("peer:198.51.100.4", &token, solution, 1_001)
        }));
    }
    let successes = workers
        .into_iter()
        .map(|worker| worker.join().expect("verification worker"))
        .filter(Result::is_ok)
        .count();
    assert_eq!(successes, 1);
    assert_eq!(gate.active_challenges(), 0);
    assert_eq!(
        gate.verify_at("peer:198.51.100.4", challenge.token(), solution, 1_001,),
        Err(SentinelError::ReplayOrUnknownChallenge)
    );

    let expired = gate
        .issue_at("peer:198.51.100.4".to_string(), 2_000)
        .expect("expiring challenge");
    let expired_solution = solve(&expired);
    assert_eq!(
        gate.verify_at(
            "peer:198.51.100.4",
            expired.token(),
            expired_solution,
            2_030,
        ),
        Err(SentinelError::ExpiredChallenge)
    );
    assert_eq!(gate.active_challenges(), 0);
}

#[test]
fn capacity_keys_configuration_and_composed_sentinel_fail_closed() {
    assert!(ProofOfWorkGate::try_new(b"weak", ProofOfWorkConfig::default()).is_err());
    assert!(ProofOfWorkConfig::try_new(7, Duration::from_secs(30), 1).is_err());
    assert!(ProofOfWorkConfig::try_new(8, Duration::from_secs(4), 1).is_err());

    let gate = test_gate(1);
    gate.issue_at("peer:a".to_string(), 100)
        .expect("first challenge");
    assert_eq!(
        gate.issue_at("peer:b".to_string(), 100),
        Err(SentinelError::CapacityReached)
    );
    assert!(gate.issue_at("peer:b".to_string(), 131).is_ok());

    let sentinel = ThreatSentinel::try_new(
        KEY,
        SentinelPolicy::default(),
        ProofOfWorkConfig::try_new(8, Duration::from_secs(30), 4).expect("PoW config"),
    )
    .expect("Sentinel");
    assert!(
        sentinel
            .assess("peer:normal", observation(30, 1, 1, 4, 2, 0))
            .expect("normal assessment")
            .challenge()
            .is_none()
    );
    assert!(
        sentinel
            .assess("peer:suspicious", observation(25, 20, 8, 3, 1, 0))
            .expect("suspicious assessment")
            .challenge()
            .is_some()
    );
}
