use super::*;
use crate::auth::passkey::PublicKeyCredential;

type Edit = fn(&mut PublicKeyCredential);

fn handle(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}
/// A second valid P-256 key, so a credential can differ only by its public key.
fn other_public_key() -> Vec<u8> {
    fixture_for(&"A".repeat(43)).public_key
}
/// Issue an assertion challenge for `passkey` and sign a matching response.
async fn begin(
    manager: &SharedPasskeyAuth<MemoryStore>,
    bound: &PasskeyBinding,
    passkey: &Passkey,
    fixture: &RegistrationFixture,
) -> (String, PublicKeyCredential) {
    let (_, challenge) = manager
        .start_authenticate(bound, std::slice::from_ref(passkey))
        .await
        .unwrap();
    let assertion =
        assertion_for_challenge(&challenge, passkey, &fixture.key_pair, "localhost", 0x05, 2);
    (challenge, assertion)
}
fn passkey_with_id(passkey: &Passkey, credential_id: Vec<u8>) -> Passkey {
    Passkey {
        credential_id,
        ..passkey.clone()
    }
}

#[tokio::test]
async fn assertion_round_trip_checks_the_user_handle_and_is_single_use() {
    let store = MemoryStore::new();
    let manager = manager(&store);
    let bound = binding();
    let (passkey, fixture) = registered(&manager, &bound).await;
    let (options, challenge) = manager
        .start_authenticate(&bound, std::slice::from_ref(&passkey))
        .await
        .unwrap();
    assert_eq!(options.public_key.challenge, challenge);
    assert_eq!(options.public_key.allow_credentials.len(), 1);
    assert_eq!(
        options.public_key.allow_credentials[0].id,
        handle(&[10, 20, 30, 40])
    );
    let assertion = assertion_for_challenge(
        &challenge,
        &passkey,
        &fixture.key_pair,
        "localhost",
        0x05,
        2,
    );
    for wrong in [handle(&[9; 32]), "!!!!".to_owned()] {
        assert_eq!(
            manager
                .finish_authenticate(
                    &bound,
                    &assertion,
                    &challenge,
                    passkey.clone(),
                    Some(&wrong)
                )
                .await
                .unwrap_err(),
            Error::Rejected
        );
    }
    let mut other = passkey.clone();
    other.credential_id = vec![10, 20, 30, 41];
    assert_eq!(
        manager
            .finish_authenticate(&bound, &assertion, &challenge, other, None)
            .await
            .unwrap_err(),
        Error::Rejected,
        "the asserted credential must be the supplied passkey"
    );
    assert_eq!(store.consumed(), 1, "only the registration was consumed");
    let updated = manager
        .finish_authenticate(
            &bound,
            &assertion,
            &challenge,
            passkey.clone(),
            Some(&handle(&[7; 32])),
        )
        .await
        .unwrap();
    assert_eq!(updated.sign_count, 2);
    assert_eq!(updated.credential_id, passkey.credential_id);
    assert_eq!(
        manager
            .finish_authenticate(&bound, &assertion, &challenge, passkey, None)
            .await
            .unwrap_err(),
        Error::Rejected
    );
}

#[tokio::test]
async fn assertion_responses_and_handles_are_bounded_before_consumption() {
    let store = MemoryStore::new();
    let manager = manager(&store);
    // The longest accepted user handle (64 bytes) encodes to 86 characters.
    let bound = PasskeyBinding::new("tenant-a", "subject-a", "session-a", vec![7; 64]).unwrap();
    let (passkey, fixture) = registered(&manager, &bound).await;
    let (challenge, assertion) = begin(&manager, &bound, &passkey, &fixture).await;
    let edited = |edit: Edit| {
        let mut credential = assertion.clone();
        edit(&mut credential);
        credential
    };
    for (credential, user_handle) in [
        (
            edited(|c| c.response.authenticator_data = "A".repeat(513)),
            None,
        ),
        (edited(|c| c.response.signature = "A".repeat(257)), None),
        (edited(|c| c.id = "A".repeat(1_365)), None),
        (assertion.clone(), Some("A".repeat(87))),
    ] {
        assert_eq!(
            manager
                .finish_authenticate(
                    &bound,
                    &credential,
                    &challenge,
                    passkey.clone(),
                    user_handle.as_deref()
                )
                .await
                .unwrap_err(),
            Error::InvalidInput
        );
    }
    assert_eq!(store.consumed(), 1, "only the registration was consumed");
    let longest = handle(&[7; 64]);
    assert_eq!(longest.len(), 86);
    assert!(
        manager
            .finish_authenticate(
                &bound,
                &assertion,
                &challenge,
                passkey.clone(),
                Some(&longest)
            )
            .await
            .is_ok()
    );
    // At each size limit the response is consumed, then fails verification.
    let limits: [Edit; 2] = [
        |c| c.response.authenticator_data = "A".repeat(512),
        |c| c.response.signature = "A".repeat(256),
    ];
    for edit in limits {
        let (challenge, mut assertion) = begin(&manager, &bound, &passkey, &fixture).await;
        edit(&mut assertion);
        assert_eq!(
            manager
                .finish_authenticate(&bound, &assertion, &challenge, passkey.clone(), None)
                .await
                .unwrap_err(),
            Error::Rejected
        );
    }
    assert_eq!(store.consumed(), 4);
}

#[tokio::test]
async fn allowlists_are_bounded_unique_and_hold_valid_keys() {
    let store = MemoryStore::new();
    let manager = manager(&store);
    let bound = binding();
    let (passkey, _) = registered(&manager, &bound).await;
    assert_eq!(
        manager.start_authenticate(&bound, &[]).await.unwrap_err(),
        Error::InvalidInput
    );
    let many: Vec<Passkey> = (0..33_u8)
        .map(|index| passkey_with_id(&passkey, vec![index; 4]))
        .collect();
    let (options, _) = manager
        .start_authenticate(&bound, &many[..32])
        .await
        .unwrap();
    assert_eq!(options.public_key.allow_credentials.len(), 32);
    assert_eq!(
        manager.start_authenticate(&bound, &many).await.unwrap_err(),
        Error::InvalidInput
    );
    let (options, _) = manager
        .start_authenticate(&bound, &many[..2])
        .await
        .unwrap();
    assert_eq!(options.public_key.allow_credentials.len(), 2);
    let rekeyed = Passkey {
        public_key: other_public_key(),
        ..passkey.clone()
    };
    for duplicate in [
        [passkey.clone(), passkey.clone()],
        [passkey.clone(), rekeyed],
    ] {
        assert_eq!(
            manager
                .start_authenticate(&bound, &duplicate)
                .await
                .unwrap_err(),
            Error::InvalidInput
        );
    }
    let mut off_curve = vec![4];
    off_curve.extend_from_slice(&[1; 64]);
    let mut wrong_tag = passkey.public_key.clone();
    wrong_tag[0] = 3;
    let invalid = [
        passkey_with_id(&passkey, vec![]),
        passkey_with_id(&passkey, vec![1; 1_024]),
        Passkey {
            public_key: passkey.public_key[..33].to_vec(),
            ..passkey.clone()
        },
        Passkey {
            public_key: wrong_tag,
            ..passkey.clone()
        },
        Passkey {
            public_key: off_curve,
            ..passkey.clone()
        },
    ];
    let issued = store.issued();
    for candidate in invalid {
        assert_eq!(
            manager
                .start_authenticate(&bound, &[candidate])
                .await
                .unwrap_err(),
            Error::InvalidInput
        );
    }
    assert_eq!(store.issued(), issued, "invalid allowlists are not issued");
    assert!(
        manager
            .start_authenticate(&bound, &[passkey_with_id(&passkey, vec![1; 1_023])])
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn assertions_must_name_the_allowed_credential_snapshot() {
    let store = MemoryStore::new();
    let manager = manager(&store);
    let bound = binding();
    let (passkey, fixture) = registered(&manager, &bound).await;
    // A stale counter is a different snapshot from the one that was allowed.
    let mut stale = passkey.clone();
    stale.sign_count = 0;
    let (_, challenge) = manager
        .start_authenticate(&bound, std::slice::from_ref(&passkey))
        .await
        .unwrap();
    let assertion =
        assertion_for_challenge(&challenge, &stale, &fixture.key_pair, "localhost", 0x05, 2);
    assert_eq!(
        manager
            .finish_authenticate(&bound, &assertion, &challenge, stale, None)
            .await
            .unwrap_err(),
        Error::Rejected
    );
    // Another credential in the allowlist does not admit this one.
    let other = passkey_with_id(&passkey, vec![1, 2, 3]);
    let (_, challenge) = manager
        .start_authenticate(&bound, std::slice::from_ref(&other))
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
    assert_eq!(
        manager
            .finish_authenticate(&bound, &assertion, &challenge, passkey.clone(), None)
            .await
            .unwrap_err(),
        Error::Rejected
    );
    assert_eq!(store.confirmed(), 1, "only the registration was confirmed");
    // With both allowed, the listed snapshot completes.
    let (_, challenge) = manager
        .start_authenticate(&bound, &[other, passkey.clone()])
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
    assert_eq!(
        manager
            .finish_authenticate(&bound, &assertion, &challenge, passkey, None)
            .await
            .unwrap()
            .sign_count,
        2
    );
}

#[tokio::test]
async fn consumed_assertions_reported_with_another_kind_are_corrupt() {
    let store = MemoryStore::new();
    let bound = binding();
    let (passkey, fixture) = registered(&manager(&store), &bound).await;
    let tampered = manager(&store.tampered(Tamper::Kind));
    let (_, challenge) = tampered
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
    assert_eq!(
        tampered
            .finish_authenticate(&bound, &assertion, &challenge, passkey, None)
            .await
            .unwrap_err(),
        Error::Corrupt
    );
}
