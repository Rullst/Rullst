use super::*;

pub(super) async fn independent_managers_bind_registration_and_assertion(url: &str) {
    let (raw, a, b, clock) = reset(url).await;
    let first = SharedPasskeyAuth::new(&auth_config(), a.clone()).unwrap();
    let second = SharedPasskeyAuth::new(&auth_config(), b.clone()).unwrap();
    let bound = binding();
    let (options, challenge) = first
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    let (_, overlapping) = first
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    assert_ne!(
        challenge, overlapping,
        "independent outstanding challenges must not alias"
    );
    assert_eq!(
        options.public_key.user.id,
        URL_SAFE_NO_PAD.encode(vec![7; 32])
    );
    let fixture = registered_fixture(&challenge);
    for wrong in [
        PasskeyBinding::new("tenant-b", "subject-a", "session-a", vec![7; 32]).unwrap(),
        PasskeyBinding::new("tenant-a", "subject-b", "session-a", vec![7; 32]).unwrap(),
        PasskeyBinding::new("tenant-a", "subject-a", "session-b", vec![7; 32]).unwrap(),
        PasskeyBinding::new("tenant-a", "subject-a", "session-a", vec![8; 32]).unwrap(),
    ] {
        assert!(matches!(
            second
                .finish_register(&wrong, &fixture.credential, &challenge)
                .await,
            Err(Error::Rejected)
        ));
    }
    let weak =
        SharedPasskeyAuth::new(&auth_config().require_user_verification(false), b.clone()).unwrap();
    assert!(matches!(
        weak.finish_register(&bound, &fixture.credential, &challenge)
            .await,
        Err(Error::Rejected)
    ));
    let passkey = second
        .finish_register(&bound, &fixture.credential, &challenge)
        .await
        .unwrap();
    assert_eq!(passkey.public_key, fixture.public_key);
    assert!(matches!(
        first
            .finish_register(&bound, &fixture.credential, &challenge)
            .await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        first.start_authenticate(&bound, &[]).await,
        Err(Error::InvalidInput)
    ));
    let (_, challenge) = first
        .start_authenticate(&bound, std::slice::from_ref(&passkey))
        .await
        .unwrap();
    let assertion = assertion_for_challenge(
        &challenge,
        &passkey,
        &fixture.key_pair,
        "localhost",
        0x05,
        2,
    );
    assert!(matches!(
        second
            .finish_authenticate(
                &bound,
                &assertion,
                &challenge,
                passkey.clone(),
                Some(&URL_SAFE_NO_PAD.encode(vec![9; 32]))
            )
            .await,
        Err(Error::Rejected)
    ));
    let updated = second
        .finish_authenticate(
            &bound,
            &assertion,
            &challenge,
            passkey.clone(),
            Some(&URL_SAFE_NO_PAD.encode(vec![7; 32])),
        )
        .await
        .unwrap();
    assert_eq!(updated.sign_count, 2);
    assert!(
        first
            .finish_authenticate(&bound, &assertion, &challenge, passkey.clone(), None)
            .await
            .is_err()
    );
    // Finishing after expiry cannot turn a valid signature into a login.
    let (_, challenge) = first
        .start_authenticate(&bound, std::slice::from_ref(&updated))
        .await
        .unwrap();
    let assertion = assertion_for_challenge(
        &challenge,
        &updated,
        &fixture.key_pair,
        "localhost",
        0x05,
        3,
    );
    clock.set(1030);
    assert!(matches!(
        second
            .finish_authenticate(&bound, &assertion, &challenge, updated, None)
            .await,
        Err(Error::Expired)
    ));
    a.close().await;
    b.close().await;
    raw.close().await;
}

pub(super) async fn credential_snapshot_and_parallel_completion(url: &str) {
    let (raw, a, b, _) = reset(url).await;
    let first = SharedPasskeyAuth::new(&auth_config(), a.clone()).unwrap();
    let second = SharedPasskeyAuth::new(&auth_config(), b.clone()).unwrap();
    let bound = binding();
    let (_, challenge) = first
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    let fixture = registered_fixture(&challenge);
    let (left, right) = tokio::join!(
        first.finish_register(&bound, &fixture.credential, &challenge),
        second.finish_register(&bound, &fixture.credential, &challenge)
    );
    assert_ne!(left.is_ok(), right.is_ok());
    let passkey = left.or(right).unwrap();
    let (_, challenge) = first
        .start_authenticate(&bound, std::slice::from_ref(&passkey))
        .await
        .unwrap();
    let assertion = assertion_for_challenge(
        &challenge,
        &passkey,
        &fixture.key_pair,
        "localhost",
        0x05,
        3,
    );
    let mut changed = passkey.clone();
    changed.sign_count = 2;
    assert!(matches!(
        second
            .finish_authenticate(&bound, &assertion, &challenge, changed, None)
            .await,
        Err(Error::Rejected)
    ));
    assert!(
        matches!(
            first
                .finish_authenticate(&bound, &assertion, &challenge, passkey.clone(), None)
                .await,
            Err(Error::Rejected)
        ),
        "failed verification consumes the ceremony"
    );
    let (_, challenge) = first
        .start_authenticate(&bound, std::slice::from_ref(&passkey))
        .await
        .unwrap();
    let assertion = assertion_for_challenge(
        &challenge,
        &passkey,
        &fixture.key_pair,
        "localhost",
        0x05,
        2,
    );
    let (left, right) = tokio::join!(
        first.finish_authenticate(&bound, &assertion, &challenge, passkey.clone(), None),
        second.finish_authenticate(&bound, &assertion, &challenge, passkey.clone(), None)
    );
    assert_ne!(left.is_ok(), right.is_ok());
    // Malformed/oversized responses do not invoke unbounded decoders.
    let (_, challenge) = first
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    let mut bad: RegisterPublicKeyCredential = registered_fixture(&challenge).credential;
    bad.response.client_data_json = "A".repeat(8193);
    assert!(matches!(
        first.finish_register(&bound, &bad, &challenge).await,
        Err(Error::InvalidInput)
    ));
    a.close().await;
    b.close().await;
    raw.close().await;
}

pub(super) async fn completion_rechecks_expiry_after_cryptography(url: &str) {
    struct ExpireAfterConsume {
        inner: Store,
        clock: TestClock,
    }
    impl PasskeyCeremonyStore for ExpireAfterConsume {
        fn config(&self) -> &CeremonyStoreConfig {
            self.inner.config()
        }
        fn durability(&self) -> CeremonyDurability {
            self.inner.durability()
        }
        async fn issue(&self, intent: &CeremonyIntent) -> Result<(), Error> {
            self.inner.issue(intent).await
        }
        async fn consume(
            &self,
            challenge: [u8; 32],
            binding: [u8; 32],
            kind: CeremonyKind,
        ) -> Result<ConsumedCeremony, Error> {
            let consumed = self.inner.consume(challenge, binding, kind).await?;
            self.clock.set(consumed.expires_at());
            Ok(consumed)
        }
        async fn confirm(&self, consumed: &ConsumedCeremony) -> Result<(), Error> {
            self.inner.confirm(consumed).await
        }
    }
    let (raw, a, b, clock) = reset(url).await;
    let first = SharedPasskeyAuth::new(&auth_config(), a.clone()).unwrap();
    let second = SharedPasskeyAuth::new(
        &auth_config(),
        ExpireAfterConsume {
            inner: b.clone(),
            clock,
        },
    )
    .unwrap();
    let bound = binding();
    let (_, challenge) = first
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    let fixture = registered_fixture(&challenge);
    assert!(matches!(
        second
            .finish_register(&bound, &fixture.credential, &challenge)
            .await,
        Err(Error::Expired)
    ));
    assert!(matches!(
        first
            .finish_register(&bound, &fixture.credential, &challenge)
            .await,
        Err(Error::Rejected)
    ));
    a.close().await;
    b.close().await;
    raw.close().await;
}
