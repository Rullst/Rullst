#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use alloc::string::ToString;
use ed25519_dalek::{Signer, SigningKey};

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; 32])
}

fn signed_update(counter: u64, firmware: &[u8]) -> (OtaManifest, Vec<u8>) {
    let manifest = OtaManifest::from_firmware("esp32-s3", "12.0.0", counter, firmware).unwrap();
    let signature = signing_key().sign(&manifest.signing_bytes().unwrap());
    (manifest, signature.to_bytes().to_vec())
}

fn manager(counter: u64) -> OtaManager {
    OtaManager::new_with_trusted_key(
        "esp32-s3",
        "11.0.0",
        counter,
        signing_key().verifying_key().to_bytes(),
    )
    .unwrap()
}

#[test]
fn signed_update_moves_only_to_the_inactive_partition_after_full_verification() {
    let firmware = b"signed firmware bytes";
    let (manifest, signature) = signed_update(2, firmware);
    assert_eq!(manifest.target(), "esp32-s3");
    assert_eq!(manifest.version(), "12.0.0");
    assert_eq!(manifest.rollback_counter(), 2);
    assert_eq!(manifest.firmware_len(), firmware.len() as u64);
    let expected_hash: [u8; 32] = Sha256::digest(firmware).into();
    assert_eq!(manifest.firmware_sha256(), expected_hash);

    let mut manager = manager(1);
    assert_eq!(manager.rollback_counter(), 1);
    assert!(manager.pending_manifest().is_none());
    manager
        .verify_update(&manifest, firmware, &signature)
        .unwrap();
    assert_eq!(manager.status, OtaStatus::Verified);
    assert_eq!(manager.pending_manifest(), Some(&manifest));

    let commit = manager.commit_verified_update().unwrap();
    assert_eq!(commit.target_partition(), BootPartition::PartitionB);
    assert_eq!(commit.version(), "12.0.0");
    assert_eq!(commit.rollback_counter(), 2);
    assert_eq!(manager.current_partition, BootPartition::PartitionB);
    assert_eq!(manager.firmware_version, "12.0.0");
    assert_eq!(manager.rollback_counter(), 2);
    assert_eq!(manager.status, OtaStatus::Idle);
    assert!(matches!(
        manager.commit_verified_update(),
        Err(OtaError::NoVerifiedUpdate)
    ));
    assert_eq!(
        BootPartition::PartitionB.opposite(),
        BootPartition::PartitionA
    );
}

#[test]
fn manifests_keys_signatures_hashes_targets_and_counters_fail_closed() {
    assert!(matches!(
        OtaManifest::from_firmware("", "1", 1, b"firmware"),
        Err(OtaError::EmptyTarget)
    ));
    assert!(matches!(
        OtaManifest::from_firmware("target", "", 1, b"firmware"),
        Err(OtaError::EmptyVersion)
    ));
    assert!(matches!(
        OtaManifest::from_firmware("target", "1", 1, b""),
        Err(OtaError::EmptyFirmware)
    ));
    assert!(matches!(
        OtaManager::new_with_trusted_key("target", "1", 0, [0_u8; 32]),
        Err(OtaError::InvalidTrustedKey)
    ));

    let firmware = b"firmware";
    let (manifest, signature) = signed_update(2, firmware);

    let mut wrong_target = manifest.clone();
    wrong_target.target = "other-board".to_string();
    assert!(matches!(
        manager(1).verify_update(&wrong_target, firmware, &signature),
        Err(OtaError::TargetMismatch)
    ));

    assert!(matches!(
        manager(2).verify_update(&manifest, firmware, &signature),
        Err(OtaError::RollbackDetected {
            current: 2,
            proposed: 2
        })
    ));

    let mut wrong_length = manifest.clone();
    wrong_length.firmware_len += 1;
    assert!(matches!(
        manager(1).verify_update(&wrong_length, firmware, &signature),
        Err(OtaError::FirmwareLengthMismatch { .. })
    ));

    let mut wrong_hash = manifest.clone();
    wrong_hash.firmware_sha256 = [0_u8; 32];
    assert!(matches!(
        manager(1).verify_update(&wrong_hash, firmware, &signature),
        Err(OtaError::FirmwareHashMismatch)
    ));

    assert!(matches!(
        manager(1).verify_update(&manifest, firmware, &[0_u8; 63]),
        Err(OtaError::InvalidSignatureEncoding)
    ));
    assert!(matches!(
        manager(1).verify_update(&manifest, firmware, &[0_u8; 64]),
        Err(OtaError::SignatureInvalid)
    ));
}

#[test]
#[allow(deprecated)]
fn legacy_apis_and_every_typed_error_remain_explicit() {
    assert!(matches!(
        OtaManager::new("1.0.0"),
        Err(OtaError::LegacyApiUnsupported { .. })
    ));
    let mut manager = manager(0);
    assert!(matches!(
        manager.verify_signature(b"payload", &[0_u8; 64]),
        Err(OtaError::LegacyApiUnsupported { .. })
    ));
    assert!(matches!(
        manager.commit_update("2.0.0"),
        Err(OtaError::LegacyApiUnsupported { .. })
    ));
    assert_eq!(manager.status, OtaStatus::Failed);

    let errors = [
        OtaError::EmptyTarget,
        OtaError::EmptyVersion,
        OtaError::EmptyFirmware,
        OtaError::ManifestFieldTooLong,
        OtaError::FirmwareTooLarge,
        OtaError::InvalidTrustedKey,
        OtaError::InvalidSignatureEncoding,
        OtaError::SignatureInvalid,
        OtaError::FirmwareLengthMismatch {
            expected: 1,
            actual: 2,
        },
        OtaError::FirmwareHashMismatch,
        OtaError::TargetMismatch,
        OtaError::RollbackDetected {
            current: 2,
            proposed: 1,
        },
        OtaError::NoVerifiedUpdate,
        OtaError::LegacyApiUnsupported {
            replacement: "replacement",
        },
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}
