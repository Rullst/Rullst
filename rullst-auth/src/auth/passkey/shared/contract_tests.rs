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

#[test]
fn accessors_return_the_validated_values_and_debug_output_is_redacted() {
    let config = CeremonyStoreConfig::new("epoch-1", 8, 30).unwrap();
    assert_eq!(config.epoch(), "epoch-1");
    assert_eq!(config.capacity(), 8);
    assert_eq!(config.lifetime_seconds(), 30);
    assert_eq!(
        format!("{config:?}"),
        r#"CeremonyStoreConfig { epoch: "[redacted]", capacity: 8, lifetime_seconds: 30 }"#
    );
    let binding = PasskeyBinding::new("tenant-a", "subject-a", "session-a", vec![1; 32]).unwrap();
    assert_eq!(binding.tenant(), "tenant-a");
    assert_eq!(binding.subject(), "subject-a");
    assert_eq!(format!("{binding:?}"), "PasskeyBinding([redacted])");
    let intent = CeremonyIntent::new(
        [1; 32],
        [2; 32],
        CeremonyKind::Authentication,
        vec![[3; 32], [4; 32]],
    )
    .unwrap();
    assert_eq!(intent.credentials(), &[[3; 32], [4; 32]]);
    assert_eq!(
        format!("{intent:?}"),
        "CeremonyIntent { kind: Authentication, .. }"
    );
    let consumed = ConsumedCeremony::from_stored(intent, 1_000, 1_300).unwrap();
    assert_eq!(consumed.issued_at(), 1_000);
    assert_eq!(consumed.expires_at(), 1_300);
    assert_eq!(format!("{consumed:?}"), "ConsumedCeremony([redacted])");
}

#[test]
fn stored_ceremony_and_reference_bounds_are_inclusive() {
    let intent =
        || CeremonyIntent::new([1; 32], [2; 32], CeremonyKind::Registration, vec![]).unwrap();
    for (issued, expires) in [
        (0, 1),
        (0, 600),
        (1_000, 1_600),
        (253_402_300_199, 253_402_300_799),
    ] {
        let consumed = ConsumedCeremony::from_stored(intent(), issued, expires).unwrap();
        assert_eq!(
            (consumed.issued_at(), consumed.expires_at()),
            (issued, expires)
        );
    }
    for (issued, expires) in [
        (-1, 10),
        (10, 10),
        (1_000, 999),
        (0, 601),
        (253_402_300_790, 253_402_300_800),
    ] {
        assert_eq!(
            ConsumedCeremony::from_stored(intent(), issued, expires).unwrap_err(),
            PasskeyCeremonyError::Corrupt
        );
    }
    let longest = "r".repeat(128);
    assert!(PasskeyBinding::new(longest.as_str(), "subject", "session", vec![1; 64]).is_ok());
    assert!(CeremonyStoreConfig::new(longest.as_str(), 1, 1).is_ok());
}

#[test]
fn system_clock_reports_the_current_unix_time() {
    let now = SystemCeremonyClock.now().unwrap();
    let reference = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    assert!(
        (now - reference).abs() <= 5,
        "clock is off by {}",
        now - reference
    );
}
