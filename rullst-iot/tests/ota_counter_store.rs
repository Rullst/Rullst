#![allow(clippy::unwrap_used, clippy::expect_used)]

use ed25519_dalek::{Signer, SigningKey};
use rullst_iot::{
    BootPartition, OtaError, OtaManager, OtaManifest, OtaStatus, RollbackCounterError,
    RollbackCounterStore,
};

struct MemoryCounterStore {
    value: u64,
    load_error: Option<RollbackCounterError>,
    commit_error: Option<RollbackCounterError>,
}

impl MemoryCounterStore {
    fn new(value: u64) -> Self {
        Self {
            value,
            load_error: None,
            commit_error: None,
        }
    }
}

impl RollbackCounterStore for MemoryCounterStore {
    fn load(&mut self) -> Result<u64, RollbackCounterError> {
        if let Some(error) = self.load_error.take() {
            return Err(error);
        }
        Ok(self.value)
    }

    fn compare_and_set(
        &mut self,
        expected: u64,
        proposed: u64,
    ) -> Result<(), RollbackCounterError> {
        if let Some(error) = self.commit_error.take() {
            return Err(error);
        }
        if self.value != expected {
            return Err(RollbackCounterError::Conflict {
                expected,
                actual: self.value,
            });
        }
        if proposed <= self.value {
            return Err(RollbackCounterError::NonMonotonic {
                current: self.value,
                proposed,
            });
        }
        self.value = proposed;
        Ok(())
    }
}

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[73_u8; 32])
}

fn manager(store: &mut MemoryCounterStore) -> OtaManager {
    OtaManager::new_with_counter_store(
        "counter-test-board",
        "12.0.0",
        signing_key().verifying_key().to_bytes(),
        store,
    )
    .unwrap()
}

fn signed_update(counter: u64) -> (OtaManifest, Vec<u8>, Vec<u8>) {
    let firmware = format!("firmware-counter-{counter}").into_bytes();
    let manifest =
        OtaManifest::from_firmware("counter-test-board", "12.1.0", counter, &firmware).unwrap();
    let signature = signing_key()
        .sign(&manifest.signing_bytes().unwrap())
        .to_bytes()
        .to_vec();
    (manifest, firmware, signature)
}

#[test]
fn durable_counter_is_reloaded_after_restart_and_rejects_replay() {
    let mut store = MemoryCounterStore::new(7);
    let (manifest, firmware, signature) = signed_update(8);
    let mut first_boot = manager(&mut store);

    first_boot
        .verify_update(&manifest, &firmware, &signature)
        .unwrap();
    assert_eq!(
        first_boot.verified_target_partition().unwrap(),
        BootPartition::PartitionB
    );
    let receipt = first_boot
        .commit_verified_update_with_store(&mut store)
        .unwrap();
    assert_eq!(receipt.rollback_counter(), 8);
    assert_eq!(store.value, 8);

    let mut after_restart = manager(&mut store);
    assert_eq!(after_restart.rollback_counter(), 8);
    assert_eq!(
        after_restart.verify_update(&manifest, &firmware, &signature),
        Err(OtaError::RollbackDetected {
            current: 8,
            proposed: 8,
        })
    );
}

#[test]
fn unavailable_store_preserves_verified_state_and_allows_retry() {
    let mut store = MemoryCounterStore::new(20);
    let (manifest, firmware, signature) = signed_update(21);
    let mut ota = manager(&mut store);
    ota.verify_update(&manifest, &firmware, &signature).unwrap();
    store.commit_error = Some(RollbackCounterError::Unavailable);

    assert_eq!(
        ota.commit_verified_update_with_store(&mut store),
        Err(OtaError::RollbackCounterStore(
            RollbackCounterError::Unavailable
        ))
    );
    assert_eq!(ota.status, OtaStatus::Verified);
    assert_eq!(ota.current_partition, BootPartition::PartitionA);
    assert_eq!(ota.firmware_version, "12.0.0");
    assert_eq!(ota.rollback_counter(), 20);
    assert_eq!(ota.pending_manifest(), Some(&manifest));
    assert_eq!(store.value, 20);

    ota.commit_verified_update_with_store(&mut store).unwrap();
    assert_eq!(store.value, 21);
    assert!(ota.pending_manifest().is_none());
}

#[test]
fn stale_manager_detects_conflict_without_mutating_local_state() {
    let mut store = MemoryCounterStore::new(30);
    let mut stale = manager(&mut store);
    let (manifest, firmware, signature) = signed_update(32);
    stale
        .verify_update(&manifest, &firmware, &signature)
        .unwrap();
    store.value = 31;

    assert_eq!(
        stale.commit_verified_update_with_store(&mut store),
        Err(OtaError::RollbackCounterStore(
            RollbackCounterError::Conflict {
                expected: 30,
                actual: 31,
            }
        ))
    );
    assert_eq!(stale.status, OtaStatus::Verified);
    assert_eq!(stale.current_partition, BootPartition::PartitionA);
    assert_eq!(stale.rollback_counter(), 30);
    assert_eq!(stale.pending_manifest(), Some(&manifest));
    assert_eq!(store.value, 31);
}

#[test]
fn load_and_transition_failures_are_typed_and_bounded() {
    let mut store = MemoryCounterStore::new(40);
    store.load_error = Some(RollbackCounterError::CorruptState);
    assert!(matches!(
        OtaManager::new_with_counter_store(
            "counter-test-board",
            "12.0.0",
            signing_key().verifying_key().to_bytes(),
            &mut store,
        ),
        Err(OtaError::RollbackCounterStore(
            RollbackCounterError::CorruptState
        ))
    ));

    let error = store.compare_and_set(40, 40).unwrap_err();
    assert_eq!(
        error,
        RollbackCounterError::NonMonotonic {
            current: 40,
            proposed: 40,
        }
    );
    assert!(!error.to_string().is_empty());
}
