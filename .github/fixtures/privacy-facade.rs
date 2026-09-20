use rullst::privacy::{age_assurance::*, consent::*};

struct Clock;
impl AgeClock for Clock {
    fn now(&self) -> Result<i64, AgeError> {
        Ok(1000)
    }
}
impl ConsentClock for Clock {
    fn now(&self) -> Result<i64, ConsentError> {
        Ok(1000)
    }
}

#[test]
fn facade_keeps_replay_and_withdrawal_after_reopening_durable_stores() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = rullst::async_runtime::tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let policy = AgePolicy::new("policy-v1", RiskLevel::Low, 18).unwrap();
        let subject =
            SubjectBinding::new("student", "school", "session", "academy", "dashboard").unwrap();
        let issued =
            AgeChallenge::issue(&policy, subject.clone(), AgeMethod::SelfDeclaration, 1000)
                .unwrap();
        // Deterministic test key, never an application credential.
        let transport = ChallengeTokens::new("test-key", &[7; 32]).unwrap();
        let sealed = transport.seal(&issued).unwrap();
        let challenge = transport
            .open_with_clock(&sealed, &policy, &subject, &Clock)
            .unwrap();
        let replay_path = directory.path().join("age.sqlite");
        let replay = SqliteReplayStore::open(&replay_path, 8).await.unwrap();
        let gate = DeclarationGate::new(replay.clone()).unwrap();
        let result = gate
            .assess_with_clock(
                &policy,
                &subject,
                &challenge,
                AgeDeclaration::MeetsThreshold,
                &Clock,
            )
            .await
            .unwrap();
        assert_eq!(result.decision(), AgeDecision::Allowed);
        assert_eq!(result.assurance(), Assurance::Declared);
        replay.close().await;
        let replay = SqliteReplayStore::open(&replay_path, 8).await.unwrap();
        let gate = DeclarationGate::new(replay.clone()).unwrap();
        assert_eq!(
            gate.assess_with_clock(
                &policy,
                &subject,
                &challenge,
                AgeDeclaration::MeetsThreshold,
                &Clock
            )
            .await,
            Err(AgeError::Replay)
        );
        replay.close().await;

        let path = directory.path().join("consent.sqlite");
        let store = SqliteConsentStore::initialize(&path, 8).await.unwrap();
        let gate = ConsentGate::new(store.clone()).unwrap();
        let subject = ConsentSubject::new("student", "school").unwrap();
        let purpose = ConsentPurpose::new("optional-greeting", "notice-v1").unwrap();
        let current = gate
            .current_with_clock(&subject, &purpose, &Clock)
            .await
            .unwrap();
        let choice =
            ConsentSubmission::new(purpose.clone(), current.revision(), ConsentChoice::Granted)
                .unwrap();
        gate.choose_with_clock(&subject, &purpose, &choice, 2000, &Clock)
            .await
            .unwrap();
        assert!(
            gate.allows_with_clock(&subject, &purpose, &Clock)
                .await
                .unwrap()
        );
        gate.withdraw_with_clock(&subject, &purpose, &Clock)
            .await
            .unwrap();
        store.close().await;
        let store = SqliteConsentStore::open(&path, 8).await.unwrap();
        let gate = ConsentGate::new(store.clone()).unwrap();
        assert!(
            !gate
                .allows_with_clock(&subject, &purpose, &Clock)
                .await
                .unwrap()
        );
        assert_eq!(
            gate.choose_with_clock(&subject, &purpose, &choice, 2000, &Clock)
                .await,
            Err(ConsentError::RevisionConflict)
        );
        store.close().await;
    });
}
