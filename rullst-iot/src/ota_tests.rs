#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use alloc::string::ToString;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

fn sign(manifest: &OtaManifest) -> Vec<u8> {
    signing_key()
        .sign(&manifest.signing_bytes().unwrap())
        .to_bytes()
        .to_vec()
}

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
    assert_eq!(manager.status(), OtaStatus::Verified);
    assert_eq!(manager.pending_manifest(), Some(&manifest));

    let commit = manager.commit_verified_update().unwrap();
    assert_eq!(commit.target_partition(), BootPartition::PartitionB);
    assert_eq!(commit.version(), "12.0.0");
    assert_eq!(commit.rollback_counter(), 2);
    // The receipt selects B for the next boot; the device still runs A.
    assert_eq!(manager.current_partition(), BootPartition::PartitionA);
    assert_eq!(manager.firmware_version(), "12.0.0");
    assert_eq!(manager.rollback_counter(), 2);
    assert_eq!(manager.status(), OtaStatus::Idle);
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
// TM-IOT-1 (IOT-04): a further update verified before the platform reboots
// into the committed bank must not target the bank that is still running.
fn a_second_update_before_reboot_never_targets_the_running_bank() {
    let mut manager = manager(1);
    for (counter, firmware) in [(2, &b"first image"[..]), (3, &b"second image"[..])] {
        let (manifest, signature) = signed_update(counter, firmware);
        manager
            .verify_update(&manifest, firmware, &signature)
            .unwrap();
        assert_eq!(
            manager.verified_target_partition().unwrap(),
            BootPartition::PartitionB
        );
        let commit = manager.commit_verified_update().unwrap();
        assert_eq!(commit.target_partition(), BootPartition::PartitionB);
        assert_eq!(manager.current_partition(), BootPartition::PartitionA);
    }

    // After the platform reboots into B and reports it, A is the target.
    manager = manager.with_running_partition(BootPartition::PartitionB);
    let (manifest, signature) = signed_update(4, b"third image");
    manager
        .verify_update(&manifest, b"third image", &signature)
        .unwrap();
    assert_eq!(
        manager.verified_target_partition().unwrap(),
        BootPartition::PartitionA
    );
}

#[test]
fn reporting_the_running_bank_discards_an_update_verified_for_the_other_one() {
    let (manifest, signature) = signed_update(2, b"image");
    let mut manager = manager(1);
    assert_eq!(manager.status(), OtaStatus::Idle);
    assert_eq!(manager.firmware_version(), "11.0.0");
    manager
        .verify_update(&manifest, b"image", &signature)
        .unwrap();
    assert_eq!(manager.status(), OtaStatus::Verified);
    assert_eq!(
        manager.verified_target_partition().unwrap(),
        BootPartition::PartitionB
    );

    let mut manager = manager.with_running_partition(BootPartition::PartitionB);
    assert_eq!(manager.current_partition(), BootPartition::PartitionB);
    assert_eq!(manager.status(), OtaStatus::Idle);
    assert!(manager.pending_manifest().is_none());
    assert!(matches!(
        manager.commit_verified_update(),
        Err(OtaError::NoVerifiedUpdate)
    ));
    manager
        .verify_update(&manifest, b"image", &signature)
        .unwrap();
    let commit = manager.commit_verified_update().unwrap();
    assert_eq!(commit.target_partition(), BootPartition::PartitionA);
    assert_eq!(manager.current_partition(), BootPartition::PartitionB);
    assert_eq!(manager.firmware_version(), "12.0.0");
    assert_eq!(manager.rollback_counter(), 2);
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

    // A publisher-signed manifest whose length or digest differs from the
    // image fails on the image; changed after signing, it fails the signature.
    let mut wrong_length = manifest.clone();
    wrong_length.firmware_len += 1;
    assert!(matches!(
        manager(1).verify_update(&wrong_length, firmware, &sign(&wrong_length)),
        Err(OtaError::FirmwareLengthMismatch { .. })
    ));
    assert!(matches!(
        manager(1).verify_update(&wrong_length, firmware, &signature),
        Err(OtaError::SignatureInvalid)
    ));

    let mut wrong_hash = manifest.clone();
    wrong_hash.firmware_sha256 = [0_u8; 32];
    assert!(matches!(
        manager(1).verify_update(&wrong_hash, firmware, &sign(&wrong_hash)),
        Err(OtaError::FirmwareHashMismatch)
    ));
    assert!(matches!(
        manager(1).verify_update(&wrong_hash, firmware, &signature),
        Err(OtaError::SignatureInvalid)
    ));

    assert!(matches!(
        manager(1).verify_update(&manifest, firmware, &[0_u8; 63]),
        Err(OtaError::InvalidSignatureEncoding)
    ));
    assert!(matches!(
        manager(1).verify_update(&manifest, firmware, &[0_u8; 64]),
        Err(OtaError::SignatureInvalid)
    ));

    // The signature is checked before the image's length and digest.
    assert!(matches!(
        manager(1).verify_update(&wrong_hash, firmware, &[0_u8; 63]),
        Err(OtaError::InvalidSignatureEncoding)
    ));
    assert!(matches!(
        manager(1).verify_update(&wrong_length, firmware, &[0_u8; 64]),
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
    assert_eq!(manager.status(), OtaStatus::Failed);

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
        OtaError::AllocationFailed,
        OtaError::MalformedManifest,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn only_a_conflict_naming_the_manifest_counter_confirms_an_uncertain_commit() {
    assert!(reports_committed_counter(
        RollbackCounterError::Conflict {
            expected: 1,
            actual: 3,
        },
        3
    ));
    assert!(reports_committed_counter(
        RollbackCounterError::NonMonotonic {
            current: 3,
            proposed: 3,
        },
        3
    ));
    assert!(!reports_committed_counter(
        RollbackCounterError::Conflict {
            expected: 1,
            actual: 2,
        },
        3
    ));
    assert!(!reports_committed_counter(
        RollbackCounterError::NonMonotonic {
            current: 4,
            proposed: 3,
        },
        3
    ));
    assert!(!reports_committed_counter(
        RollbackCounterError::Unavailable,
        3
    ));
    assert!(!reports_committed_counter(
        RollbackCounterError::CorruptState,
        3
    ));
}

#[test]
fn signing_bytes_are_allocated_at_their_exact_length() {
    let (manifest, _) = signed_update(7, b"firmware");
    let bytes = manifest.signing_bytes().unwrap();
    // Domain (23) + 4 + "esp32-s3" (8) + 4 + "12.0.0" (6) + 8 + 8 + 32.
    assert_eq!(bytes.len(), 93);
    assert_eq!(bytes.capacity(), bytes.len());
    assert_eq!(&bytes[..23], b"RULLST-OTA-MANIFEST-V1\0");
    assert_eq!(&bytes[23..27], &8_u32.to_be_bytes());
}

#[test]
fn a_streamed_update_is_downloading_until_it_finishes() {
    let firmware = b"chunked firmware image";
    let (manifest, signature) = signed_update(2, firmware);
    let mut ota = manager(1);
    let mut verifier = ota.begin_update(&manifest, &signature).unwrap();
    assert_eq!(verifier.manager.status, OtaStatus::Downloading);
    verifier.update(&firmware[..5]).unwrap();
    verifier.update(&firmware[5..]).unwrap();
    assert_eq!(verifier.manager.status, OtaStatus::Downloading);
    verifier.finish().unwrap();
    assert_eq!(ota.status(), OtaStatus::Verified);

    // The manifest a finished verifier leaves behind is the verified one.
    assert_eq!(ota.pending_manifest(), Some(&manifest));
    assert_eq!(OtaManifest::empty().firmware_len(), 0);
}
