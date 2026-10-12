//! Store-agnostic `SharedPasskeyAuth` tests over an in-memory trusted store.
use super::*;
use crate::auth::passkey::{
    Passkey, PasskeyAuth, PasskeyConfig, RegisterPublicKeyCredential,
    test_support::{
        RegistrationFixture, RegistrationOptions, assertion_for_challenge, registration_fixture,
    },
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::sync::{Arc, Mutex};

#[path = "manager_tests/authentication.rs"]
mod authentication;

use PasskeyCeremonyError as Error;

/// How the store misreports a consumed ceremony.
#[derive(Clone, Copy)]
enum Tamper {
    None,
    Kind,
    Challenge,
    Binding,
}

#[derive(Default)]
struct Ledger {
    pending: Vec<CeremonyIntent>,
    issued: usize,
    consumed: usize,
    confirmed: usize,
}

/// Single-use digest store shared by clones, with call counts and fault modes.
#[derive(Clone)]
struct MemoryStore {
    config: CeremonyStoreConfig,
    ledger: Arc<Mutex<Ledger>>,
    tamper: Tamper,
    always_conflict: bool,
}
impl MemoryStore {
    fn new() -> Self {
        Self {
            config: CeremonyStoreConfig::new("test-epoch", 8, 30).unwrap(),
            ledger: Arc::default(),
            tamper: Tamper::None,
            always_conflict: false,
        }
    }
    fn tampered(&self, tamper: Tamper) -> Self {
        Self {
            tamper,
            ..self.clone()
        }
    }
    fn with_epoch(&self, epoch: &str) -> Self {
        Self {
            config: CeremonyStoreConfig::new(epoch, 8, 30).unwrap(),
            ..self.clone()
        }
    }
    fn issued(&self) -> usize {
        self.ledger.lock().unwrap().issued
    }
    fn consumed(&self) -> usize {
        self.ledger.lock().unwrap().consumed
    }
    fn confirmed(&self) -> usize {
        self.ledger.lock().unwrap().confirmed
    }
}
impl PasskeyCeremonyStore for MemoryStore {
    fn config(&self) -> &CeremonyStoreConfig {
        &self.config
    }
    fn durability(&self) -> CeremonyDurability {
        CeremonyDurability::SharedDurable
    }
    async fn issue(&self, intent: &CeremonyIntent) -> Result<(), Error> {
        let mut ledger = self.ledger.lock().unwrap();
        ledger.issued += 1;
        if self.always_conflict
            || ledger
                .pending
                .iter()
                .any(|pending| pending.challenge() == intent.challenge())
        {
            return Err(Error::Conflict);
        }
        ledger.pending.push(intent.clone());
        Ok(())
    }
    async fn consume(
        &self,
        challenge: [u8; 32],
        binding: [u8; 32],
        kind: CeremonyKind,
    ) -> Result<ConsumedCeremony, Error> {
        let mut ledger = self.ledger.lock().unwrap();
        ledger.consumed += 1;
        let index = ledger
            .pending
            .iter()
            .position(|pending| {
                pending.challenge() == &challenge
                    && pending.binding() == &binding
                    && pending.kind() == kind
            })
            .ok_or(Error::Rejected)?;
        let stored = ledger.pending.remove(index);
        let credentials = stored.credentials().to_vec();
        let reported = match self.tamper {
            Tamper::None => stored,
            Tamper::Kind if kind == CeremonyKind::Registration => CeremonyIntent::new(
                challenge,
                binding,
                CeremonyKind::Authentication,
                vec![[9; 32]],
            )?,
            Tamper::Kind => {
                CeremonyIntent::new(challenge, binding, CeremonyKind::Registration, vec![])?
            }
            Tamper::Challenge => CeremonyIntent::new([0xEE; 32], binding, kind, credentials)?,
            Tamper::Binding => CeremonyIntent::new(challenge, [0xEE; 32], kind, credentials)?,
        };
        ConsumedCeremony::from_stored(reported, 1_000, 1_030)
    }
    async fn confirm(&self, _: &ConsumedCeremony) -> Result<(), Error> {
        self.ledger.lock().unwrap().confirmed += 1;
        Ok(())
    }
}

fn auth_config() -> PasskeyConfig {
    PasskeyConfig::new("Test", "localhost", "http://localhost")
        .with_max_pending_challenges(8)
        .with_challenge_ttl_seconds(30)
}
fn binding() -> PasskeyBinding {
    PasskeyBinding::new("tenant-a", "subject-a", "session-a", vec![7; 32]).unwrap()
}
fn manager(store: &MemoryStore) -> SharedPasskeyAuth<MemoryStore> {
    SharedPasskeyAuth::new(&auth_config(), store.clone()).unwrap()
}
/// Replace the challenge inside base64url clientDataJSON.
fn client_data_with(encoded: &str, challenge: &str) -> String {
    let bytes = URL_SAFE_NO_PAD.decode(encoded).unwrap();
    let mut data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    data["challenge"] = serde_json::Value::String(challenge.to_owned());
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&data).unwrap())
}
/// A registration response answering `challenge`.
fn fixture_for(challenge: &str) -> RegistrationFixture {
    let mut fixture = registration_fixture(
        &PasskeyAuth::new(&auth_config()).unwrap(),
        "localhost",
        RegistrationOptions::default(),
    );
    fixture.credential.response.client_data_json =
        client_data_with(&fixture.credential.response.client_data_json, challenge);
    fixture
}
/// Register a credential for `bound` and return it with its signing fixture.
async fn registered(
    manager: &SharedPasskeyAuth<MemoryStore>,
    bound: &PasskeyBinding,
) -> (Passkey, RegistrationFixture) {
    let (_, challenge) = manager
        .start_register(bound, "alice", "Alice")
        .await
        .unwrap();
    let fixture = fixture_for(&challenge);
    let passkey = manager
        .finish_register(bound, &fixture.credential, &challenge)
        .await
        .unwrap();
    (passkey, fixture)
}

#[tokio::test]
async fn registration_round_trip_is_single_use_and_bound_to_the_user_handle() {
    let store = MemoryStore::new();
    let manager = manager(&store);
    let bound = binding();
    let (options, challenge) = manager
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    assert_eq!(challenge.len(), 43);
    assert_eq!(URL_SAFE_NO_PAD.decode(&challenge).unwrap().len(), 32);
    assert_eq!(options.public_key.challenge, challenge);
    assert_eq!(options.public_key.user.id, URL_SAFE_NO_PAD.encode([7; 32]));
    assert_eq!(options.public_key.user.name, "alice");
    assert_eq!(options.public_key.user.display_name, "Alice");
    let (_, overlapping) = manager
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    assert_ne!(challenge, overlapping);
    assert_eq!(store.issued(), 2, "distinct challenges never conflict");

    let fixture = fixture_for(&challenge);
    let passkey = manager
        .finish_register(&bound, &fixture.credential, &challenge)
        .await
        .unwrap();
    assert_eq!(passkey.credential_id, vec![10, 20, 30, 40]);
    assert_eq!(passkey.public_key, fixture.public_key);
    assert_eq!(passkey.sign_count, 1);
    assert_eq!(store.confirmed(), 1);
    assert_eq!(
        manager
            .finish_register(&bound, &fixture.credential, &challenge)
            .await
            .unwrap_err(),
        Error::Rejected
    );
    let second = fixture_for(&overlapping);
    assert!(
        manager
            .finish_register(&bound, &second.credential, &overlapping)
            .await
            .is_ok()
    );
    assert_eq!(store.confirmed(), 2);
}

#[tokio::test]
async fn registration_names_are_bounded_printable_and_checked_before_issue() {
    let store = MemoryStore::new();
    let manager = manager(&store);
    let bound = binding();
    let long = "a".repeat(257);
    for bad in ["", "   ", "\t", "ali\nce", "ali\u{7f}ce", long.as_str()] {
        assert_eq!(
            manager
                .start_register(&bound, bad, "Alice")
                .await
                .unwrap_err(),
            Error::InvalidInput
        );
        assert_eq!(
            manager
                .start_register(&bound, "alice", bad)
                .await
                .unwrap_err(),
            Error::InvalidInput
        );
    }
    assert_eq!(store.issued(), 0);
    let longest = "a".repeat(256);
    let (options, _) = manager
        .start_register(&bound, longest.as_str(), longest.as_str())
        .await
        .unwrap();
    assert_eq!(options.public_key.user.name.len(), 256);
    assert_eq!(store.issued(), 1);
}

#[tokio::test]
async fn registration_bounds_responses_before_consuming_the_ceremony() {
    let store = MemoryStore::new();
    let manager = manager(&store);
    let bound = binding();
    let (_, challenge) = manager
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    let fixture = fixture_for(&challenge);
    let oversized = |edit: fn(&mut RegisterPublicKeyCredential)| {
        let mut credential = fixture.credential.clone();
        edit(&mut credential);
        credential
    };
    let invalid = [
        oversized(|c| c.response.attestation_object = "A".repeat(16_385)),
        oversized(|c| c.id = "A".repeat(1_365)),
        oversized(|c| c.raw_id = "A".repeat(1_365)),
        oversized(|c| c.r#type = "public-key-x".to_owned()),
        oversized(|c| c.response.client_data_json = "A".repeat(8_193)),
    ];
    for credential in &invalid {
        assert_eq!(
            manager
                .finish_register(&bound, credential, &challenge)
                .await
                .unwrap_err(),
            Error::InvalidInput
        );
    }
    // At each limit the response is parsed, then rejected for its content.
    for credential in [
        oversized(|c| c.id = "A".repeat(1_364)),
        oversized(|c| c.raw_id = "A".repeat(1_364)),
        oversized(|c| c.response.client_data_json = "A".repeat(8_192)),
    ] {
        assert_eq!(
            manager
                .finish_register(&bound, &credential, &challenge)
                .await
                .unwrap_err(),
            Error::Rejected
        );
    }
    assert_eq!(
        store.consumed(),
        0,
        "malformed responses never reach the store"
    );
    assert!(
        manager
            .finish_register(&bound, &fixture.credential, &challenge)
            .await
            .is_ok()
    );

    // An attestation object of exactly 16 KiB is consumed and then rejected.
    let (_, challenge) = manager
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    let mut credential = fixture_for(&challenge).credential;
    credential.response.attestation_object = "A".repeat(16_384);
    assert_eq!(
        manager
            .finish_register(&bound, &credential, &challenge)
            .await
            .unwrap_err(),
        Error::Rejected
    );
    assert_eq!(store.consumed(), 2);
}

#[tokio::test]
async fn challenges_must_match_and_be_well_formed_before_the_store_is_used() {
    let store = MemoryStore::new();
    let manager = manager(&store);
    let bound = binding();
    let (_, first) = manager
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    let (_, second) = manager
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    let answers_second = fixture_for(&second);
    assert_eq!(
        manager
            .finish_register(&bound, &answers_second.credential, &first)
            .await
            .unwrap_err(),
        Error::Rejected
    );
    let malformed = "!".repeat(43);
    let echoes_malformed = fixture_for(&malformed);
    assert_eq!(
        manager
            .finish_register(&bound, &echoes_malformed.credential, &malformed)
            .await
            .unwrap_err(),
        Error::Rejected
    );
    let short = &first[..42];
    assert_eq!(
        manager
            .finish_register(&bound, &fixture_for(short).credential, short)
            .await
            .unwrap_err(),
        Error::Rejected
    );
    assert_eq!(store.consumed(), 0);
    for challenge in [&first, &second] {
        let fixture = fixture_for(challenge);
        assert!(
            manager
                .finish_register(&bound, &fixture.credential, challenge)
                .await
                .is_ok()
        );
    }
}

#[tokio::test]
async fn bindings_relying_party_and_epoch_separate_ceremonies() {
    let store = MemoryStore::new();
    let manager = manager(&store);
    let bound = binding();
    let (_, challenge) = manager
        .start_register(&bound, "alice", "Alice")
        .await
        .unwrap();
    let fixture = fixture_for(&challenge);
    for wrong in [
        PasskeyBinding::new("tenant-b", "subject-a", "session-a", vec![7; 32]).unwrap(),
        PasskeyBinding::new("tenant-a", "subject-b", "session-a", vec![7; 32]).unwrap(),
        PasskeyBinding::new("tenant-a", "subject-a", "session-b", vec![7; 32]).unwrap(),
        PasskeyBinding::new("tenant-a", "subject-a", "session-a", vec![8; 32]).unwrap(),
    ] {
        assert_eq!(
            manager
                .finish_register(&wrong, &fixture.credential, &challenge)
                .await
                .unwrap_err(),
            Error::Rejected
        );
    }
    let weak = SharedPasskeyAuth::new(
        &auth_config().require_user_verification(false),
        store.clone(),
    )
    .unwrap();
    let other_epoch = SharedPasskeyAuth::new(&auth_config(), store.with_epoch("epoch-b")).unwrap();
    for other in [&weak, &other_epoch] {
        assert_eq!(
            other
                .finish_register(&bound, &fixture.credential, &challenge)
                .await
                .unwrap_err(),
            Error::Rejected
        );
    }
    assert_eq!(store.confirmed(), 0);
    assert!(
        manager
            .finish_register(&bound, &fixture.credential, &challenge)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn consumed_ceremonies_reported_by_the_store_are_rechecked() {
    let store = MemoryStore::new();
    let bound = binding();
    for tamper in [Tamper::Kind, Tamper::Challenge, Tamper::Binding] {
        let manager = manager(&store.tampered(tamper));
        let (_, challenge) = manager
            .start_register(&bound, "alice", "Alice")
            .await
            .unwrap();
        let fixture = fixture_for(&challenge);
        assert_eq!(
            manager
                .finish_register(&bound, &fixture.credential, &challenge)
                .await
                .unwrap_err(),
            Error::Corrupt
        );
    }
    assert_eq!(store.confirmed(), 0);
    let conflicting = MemoryStore {
        always_conflict: true,
        ..MemoryStore::new()
    };
    assert_eq!(
        manager(&conflicting)
            .start_register(&bound, "alice", "Alice")
            .await
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(conflicting.issued(), 4, "a conflict is retried four times");
}
