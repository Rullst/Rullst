use super::*;
use crate::auth::passkey::PasskeyConfig;

#[test]
fn public_bindings_and_storage_rows_are_bounded_and_debug_redacted() {
    for value in [
        "".to_owned(),
        "personal@example.test".to_owned(),
        "x".repeat(129),
    ] {
        assert!(PasskeyBinding::new(value, "subject", "session", vec![1; 32]).is_err());
    }
    for handle in [vec![], vec![0; 32], vec![1; 65]] {
        assert!(PasskeyBinding::new("tenant", "subject", "session", handle).is_err());
    }
    let binding = PasskeyBinding::new(
        "tenant-secret",
        "subject-secret",
        "session-secret",
        vec![1; 32],
    )
    .unwrap();
    let debug = format!("{binding:?}");
    for value in ["tenant-secret", "subject-secret", "session-secret"] {
        assert!(!debug.contains(value));
    }
    assert!(CeremonyIntent::new([1; 32], [2; 32], CeremonyKind::Authentication, vec![]).is_err());
    assert!(
        CeremonyIntent::new([1; 32], [2; 32], CeremonyKind::Registration, vec![[3; 32]]).is_err()
    );
    assert!(
        CeremonyIntent::new(
            [1; 32],
            [2; 32],
            CeremonyKind::Authentication,
            vec![[3; 32]; 2]
        )
        .is_err()
    );
    for (issued, expires) in [(-1, 10), (10, 10), (0, 601), (0, i64::MAX)] {
        let intent =
            CeremonyIntent::new([1; 32], [2; 32], CeremonyKind::Registration, vec![]).unwrap();
        assert!(ConsumedCeremony::from_stored(intent, issued, expires).is_err());
    }
}

#[test]
fn shared_manager_refuses_volatile_or_mismatched_configuration() {
    struct RefusingStore(CeremonyStoreConfig, CeremonyDurability);
    impl PasskeyCeremonyStore for RefusingStore {
        fn config(&self) -> &CeremonyStoreConfig {
            &self.0
        }
        fn durability(&self) -> CeremonyDurability {
            self.1
        }
        async fn issue(&self, _: &CeremonyIntent) -> Result<(), PasskeyCeremonyError> {
            Err(PasskeyCeremonyError::Unavailable)
        }
        async fn consume(
            &self,
            _: [u8; 32],
            _: [u8; 32],
            _: CeremonyKind,
        ) -> Result<ConsumedCeremony, PasskeyCeremonyError> {
            Err(PasskeyCeremonyError::Unavailable)
        }
        async fn confirm(&self, _: &ConsumedCeremony) -> Result<(), PasskeyCeremonyError> {
            Err(PasskeyCeremonyError::Unavailable)
        }
    }
    let config = PasskeyConfig::new("Test", "localhost", "http://localhost");
    for (capacity, lifetime, durability) in [
        (10000, 300, CeremonyDurability::ProcessLocal),
        (1, 300, CeremonyDurability::SharedDurable),
        (10000, 1, CeremonyDurability::SharedDurable),
    ] {
        let store = RefusingStore(
            CeremonyStoreConfig::new("epoch", capacity, lifetime).unwrap(),
            durability,
        );
        assert!(matches!(
            SharedPasskeyAuth::new(&config, store),
            Err(PasskeyCeremonyError::Configuration)
        ));
    }
    for (capacity, lifetime) in [(0, 300), (100001, 300), (1, 0), (1, 601)] {
        assert!(CeremonyStoreConfig::new("epoch", capacity, lifetime).is_err());
    }
}
