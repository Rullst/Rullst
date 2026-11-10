//! Streaming OTA verification: manifests built from received fields, images
//! verified in chunks of any size, and every rejection in chunked form.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ed25519_dalek::{Signer, SigningKey};
use rullst_iot::{BootPartition, OtaError, OtaManager, OtaManifest, OtaStatus};
use sha2::{Digest, Sha256};

const TARGET: &str = "stream-board";

fn key() -> SigningKey {
    SigningKey::from_bytes(&[41; 32])
}

fn manager(counter: u64) -> OtaManager {
    OtaManager::new_with_trusted_key(TARGET, "1.0.0", counter, key().verifying_key().to_bytes())
        .unwrap()
}

fn sign(manifest: &OtaManifest) -> Vec<u8> {
    key()
        .sign(&manifest.signing_bytes().unwrap())
        .to_bytes()
        .to_vec()
}

/// Deterministic image bytes, produced without holding the image.
struct Image {
    state: u32,
    left: u64,
}

impl Image {
    fn new(len: u64) -> Self {
        Self {
            state: 0x9e37_79b9,
            left: len,
        }
    }

    /// Fills `buffer` with the next bytes; returns how many were written.
    fn fill(&mut self, buffer: &mut [u8]) -> usize {
        let take = usize::try_from(self.left.min(buffer.len() as u64)).unwrap();
        for byte in &mut buffer[..take] {
            self.state ^= self.state << 13;
            self.state ^= self.state >> 17;
            self.state ^= self.state << 5;
            *byte = (self.state >> 24) as u8;
        }
        self.left -= take as u64;
        take
    }

    fn bytes(len: usize) -> Vec<u8> {
        let mut out = vec![0; len];
        Image::new(len as u64).fill(&mut out);
        out
    }
}

/// A signed manifest for `firmware`, built from fields as a device would.
fn signed(counter: u64, firmware: &[u8]) -> (OtaManifest, Vec<u8>) {
    let manifest = OtaManifest::new(
        TARGET,
        "2.0.0",
        counter,
        firmware.len() as u64,
        Sha256::digest(firmware).into(),
    )
    .unwrap();
    let signature = sign(&manifest);
    (manifest, signature)
}

/// Streams `firmware` split at `sizes` (cycled), finishing the update.
fn stream(
    ota: &mut OtaManager,
    manifest: &OtaManifest,
    signature: &[u8],
    firmware: &[u8],
    sizes: &[usize],
) -> Result<(), OtaError> {
    let mut verifier = ota.begin_update(manifest, signature)?;
    let mut rest = firmware;
    for size in sizes.iter().cycle() {
        if rest.is_empty() {
            break;
        }
        let (chunk, tail) = rest.split_at((*size).min(rest.len()));
        verifier.update(chunk)?;
        rest = tail;
    }
    verifier.finish()
}

#[test]
fn manifests_built_from_fields_or_parsed_match_the_publisher_manifest() {
    let firmware = Image::bytes(10_000);
    let published = OtaManifest::from_firmware(TARGET, "2.0.0", 9, &firmware).unwrap();
    let (from_fields, _) = signed(9, &firmware);
    assert_eq!(from_fields, published);

    let wire = published.signing_bytes().unwrap();
    assert_eq!(
        OtaManifest::from_signing_bytes(&wire),
        Ok(published.clone())
    );

    // Every truncation, a trailing byte and another domain are malformed.
    for len in 0..wire.len() {
        assert_eq!(
            OtaManifest::from_signing_bytes(&wire[..len]),
            Err(OtaError::MalformedManifest),
            "truncated to {len}"
        );
    }
    let mut trailing = wire.clone();
    trailing.push(0);
    assert_eq!(
        OtaManifest::from_signing_bytes(&trailing),
        Err(OtaError::MalformedManifest)
    );
    let mut domain = wire.clone();
    domain[0] ^= 1;
    assert_eq!(
        OtaManifest::from_signing_bytes(&domain),
        Err(OtaError::MalformedManifest)
    );
    // A target length that runs past the end.
    let mut long_target = wire.clone();
    long_target[23..27].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(
        OtaManifest::from_signing_bytes(&long_target),
        Err(OtaError::MalformedManifest)
    );
    // Invalid UTF-8 in the target.
    let mut utf8 = wire.clone();
    utf8[27] = 0xff;
    assert_eq!(
        OtaManifest::from_signing_bytes(&utf8),
        Err(OtaError::MalformedManifest)
    );

    // Empty fields keep their own errors.
    assert_eq!(
        OtaManifest::new("", "1", 1, 1, [0; 32]),
        Err(OtaError::EmptyTarget)
    );
    assert_eq!(
        OtaManifest::new(TARGET, "", 1, 1, [0; 32]),
        Err(OtaError::EmptyVersion)
    );
    assert_eq!(
        OtaManifest::new(TARGET, "1", 1, 0, [0; 32]),
        Err(OtaError::EmptyFirmware)
    );
    let empty_target = OtaManifest::new("x", "1", 1, 1, [0; 32])
        .unwrap()
        .signing_bytes()
        .unwrap();
    let mut no_target = empty_target[..23].to_vec();
    no_target.extend_from_slice(&0_u32.to_be_bytes());
    no_target.extend_from_slice(&empty_target[28..]);
    assert_eq!(
        OtaManifest::from_signing_bytes(&no_target),
        Err(OtaError::EmptyTarget)
    );
}

#[test]
fn any_chunking_verifies_the_same_image() {
    let firmware = Image::bytes(9_973);
    let (manifest, signature) = signed(2, &firmware);
    for sizes in [
        &[1][..],
        &[7],
        &[63],
        &[64],
        &[65],
        &[4096],
        &[9_973],
        &[20_000],
        &[0, 3, 0, 61, 64, 1, 4_000],
        &[9_972, 1],
        &[1, 9_972],
    ] {
        let mut ota = manager(1);
        assert_eq!(
            stream(&mut ota, &manifest, &signature, &firmware, sizes),
            Ok(()),
            "{sizes:?}"
        );
        assert_eq!(ota.status(), OtaStatus::Verified);
        assert_eq!(ota.pending_manifest(), Some(&manifest));
        assert_eq!(
            ota.verified_target_partition(),
            Ok(BootPartition::PartitionB)
        );
    }
}

#[test]
fn an_image_larger_than_any_buffer_streams_in_four_kib_chunks() {
    const LEN: u64 = 3 * 1024 * 1024 + 17;
    let mut hasher = Sha256::new();
    let mut image = Image::new(LEN);
    let mut buffer = [0_u8; 4096];
    loop {
        let filled = image.fill(&mut buffer);
        if filled == 0 {
            break;
        }
        hasher.update(&buffer[..filled]);
    }
    let manifest = OtaManifest::new(TARGET, "3.0.0", 5, LEN, hasher.finalize().into()).unwrap();
    let signature = sign(&manifest);

    let mut ota = manager(4);
    let mut verifier = ota.begin_update(&manifest, &signature).unwrap();
    assert_eq!(verifier.manifest(), &manifest);
    assert_eq!(verifier.remaining(), LEN);
    let mut image = Image::new(LEN);
    loop {
        let filled = image.fill(&mut buffer);
        if filled == 0 {
            break;
        }
        verifier.update(&buffer[..filled]).unwrap();
    }
    assert_eq!((verifier.received(), verifier.remaining()), (LEN, 0));
    verifier.finish().unwrap();
    assert_eq!(ota.status(), OtaStatus::Verified);
}

#[test]
fn rejections_before_the_image_return_no_verifier() {
    let firmware = Image::bytes(5_000);
    let (manifest, signature) = signed(3, &firmware);

    let other_target = OtaManifest::new("other", "2.0.0", 3, 5_000, [1; 32]).unwrap();
    let mut tampered = signature.clone();
    tampered[40] ^= 0x10;
    let wrong_key = OtaManager::new_with_trusted_key(
        TARGET,
        "1.0.0",
        1,
        SigningKey::from_bytes(&[42; 32]).verifying_key().to_bytes(),
    )
    .unwrap();
    let cases: [(OtaManager, &OtaManifest, &[u8], OtaError); 6] = [
        (
            manager(1),
            &other_target,
            &signature,
            OtaError::TargetMismatch,
        ),
        (
            manager(4),
            &manifest,
            &signature,
            OtaError::RollbackDetected {
                current: 4,
                proposed: 3,
            },
        ),
        (
            manager(3),
            &manifest,
            &signature,
            OtaError::RollbackDetected {
                current: 3,
                proposed: 3,
            },
        ),
        (
            manager(1),
            &manifest,
            &signature[..63],
            OtaError::InvalidSignatureEncoding,
        ),
        (manager(1), &manifest, &tampered, OtaError::SignatureInvalid),
        (wrong_key, &manifest, &signature, OtaError::SignatureInvalid),
    ];
    for (mut ota, manifest, signature, want) in cases {
        assert_eq!(
            ota.begin_update(manifest, signature).err(),
            Some(want.clone())
        );
        assert_eq!(ota.status(), OtaStatus::Failed, "{want:?}");
        // The slice API reports the same error for the same input.
        assert_eq!(ota.verify_update(manifest, &firmware, signature), Err(want));
    }
}

#[test]
fn image_rejections_in_chunked_form() {
    let firmware = Image::bytes(12_345);
    let (manifest, signature) = signed(2, &firmware);

    // A flipped byte in each position class of a 64-byte block, sent across
    // awkward chunk boundaries, fails the digest at `finish`.
    for (position, sizes) in [
        (0, &[1, 4096][..]),
        (63, &[63, 2]),
        (64, &[64, 1, 4096]),
        (6_000, &[777]),
        (12_344, &[12_344, 1]),
    ] {
        let mut changed = firmware.clone();
        changed[position] ^= 0x01;
        let mut ota = manager(1);
        assert_eq!(
            stream(&mut ota, &manifest, &signature, &changed, sizes),
            Err(OtaError::FirmwareHashMismatch),
            "byte {position}"
        );
        assert_eq!(ota.status(), OtaStatus::Failed);
        assert!(ota.pending_manifest().is_none());
        assert_eq!(
            ota.verified_target_partition(),
            Err(OtaError::NoVerifiedUpdate)
        );
    }

    // A short image fails at `finish`.
    let mut ota = manager(1);
    assert_eq!(
        stream(
            &mut ota,
            &manifest,
            &signature,
            &firmware[..12_344],
            &[4096]
        ),
        Err(OtaError::FirmwareLengthMismatch {
            expected: 12_345,
            actual: 12_344
        })
    );

    // A chunk past the signed length is refused before it is hashed, and the
    // verifier stays failed.
    let mut ota = manager(1);
    let mut verifier = ota.begin_update(&manifest, &signature).unwrap();
    verifier.update(&firmware[..12_000]).unwrap();
    let overflow = OtaError::FirmwareLengthMismatch {
        expected: 12_345,
        actual: 12_000 + 4_096,
    };
    assert_eq!(verifier.update(&[0; 4_096]), Err(overflow.clone()));
    assert_eq!(verifier.received(), 12_000);
    assert_eq!(verifier.update(&firmware[12_000..]), Err(overflow.clone()));
    assert_eq!(verifier.finish(), Err(overflow));
    assert_eq!(ota.status(), OtaStatus::Failed);
    assert!(ota.pending_manifest().is_none());
}

#[test]
fn the_manager_tracks_a_streamed_update_and_its_abandonment() {
    let firmware = Image::bytes(3_000);
    let (manifest, signature) = signed(2, &firmware);
    let mut ota = manager(1).with_running_partition(BootPartition::PartitionB);

    // A verified update is discarded when a new one begins.
    ota.verify_update(&manifest, &firmware, &signature).unwrap();
    {
        let mut verifier = ota.begin_update(&manifest, &signature).unwrap();
        verifier.update(&firmware[..1_000]).unwrap();
        assert_eq!(verifier.remaining(), 2_000);
        // Dropped before `finish`: an abandoned download.
    }
    assert_eq!(ota.status(), OtaStatus::Failed);
    assert!(ota.pending_manifest().is_none());
    assert_eq!(
        ota.commit_verified_update(),
        Err(OtaError::NoVerifiedUpdate)
    );

    // Downloading while it streams, then committed to the inactive bank.
    let verifier = ota.begin_update(&manifest, &signature).unwrap();
    drop(verifier);
    let mut verifier = ota.begin_update(&manifest, &signature).unwrap();
    for chunk in firmware.chunks(512) {
        verifier.update(chunk).unwrap();
    }
    verifier.finish().unwrap();
    let receipt = ota.commit_verified_update().unwrap();
    assert_eq!(receipt.target_partition(), BootPartition::PartitionA);
    assert_eq!(
        (receipt.version(), receipt.rollback_counter()),
        ("2.0.0", 2)
    );
    assert_eq!(ota.firmware_version(), "2.0.0");
}

#[test]
fn finishing_before_any_byte_is_a_length_mismatch() {
    let firmware = Image::bytes(100);
    let (manifest, signature) = signed(2, &firmware);
    let mut ota = manager(1);
    let verifier = ota.begin_update(&manifest, &signature).unwrap();
    assert_eq!(
        verifier.finish(),
        Err(OtaError::FirmwareLengthMismatch {
            expected: 100,
            actual: 0
        })
    );
    assert_eq!(ota.status(), OtaStatus::Failed);
}
