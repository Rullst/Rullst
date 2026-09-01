//! Signed firmware manifest verification and OTA boot-selection state.
//!
//! This module verifies Ed25519 signatures over a canonical manifest that binds
//! the target, version, monotonic rollback counter, firmware length, and SHA-256
//! digest. It does not download, flash, boot, or persist rollback state; platform
//! code must perform those operations and durably store the committed counter.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

const MANIFEST_DOMAIN: &[u8] = b"RULLST-OTA-MANIFEST-V1\0";

/// Active firmware boot partition.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootPartition {
    PartitionA,
    PartitionB,
}

impl BootPartition {
    /// Returns the inactive partition.
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Self::PartitionA => Self::PartitionB,
            Self::PartitionB => Self::PartitionA,
        }
    }
}

/// OTA verification state.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OtaStatus {
    Idle,
    Downloading,
    Verifying,
    Verified,
    Committing,
    Failed,
}

/// Errors returned by the signed OTA gate.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OtaError {
    EmptyTarget,
    EmptyVersion,
    EmptyFirmware,
    ManifestFieldTooLong,
    FirmwareTooLarge,
    InvalidTrustedKey,
    InvalidSignatureEncoding,
    SignatureInvalid,
    FirmwareLengthMismatch { expected: u64, actual: u64 },
    FirmwareHashMismatch,
    TargetMismatch,
    RollbackDetected { current: u64, proposed: u64 },
    NoVerifiedUpdate,
    LegacyApiUnsupported { replacement: &'static str },
}

impl fmt::Display for OtaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTarget => formatter.write_str("OTA target must not be empty"),
            Self::EmptyVersion => formatter.write_str("firmware version must not be empty"),
            Self::EmptyFirmware => formatter.write_str("firmware payload must not be empty"),
            Self::ManifestFieldTooLong => {
                formatter.write_str("OTA manifest target or version exceeds the encoded limit")
            }
            Self::FirmwareTooLarge => {
                formatter.write_str("firmware length cannot be represented by this platform")
            }
            Self::InvalidTrustedKey => formatter.write_str("trusted Ed25519 public key is invalid"),
            Self::InvalidSignatureEncoding => {
                formatter.write_str("Ed25519 signature must contain exactly 64 bytes")
            }
            Self::SignatureInvalid => formatter.write_str("Ed25519 signature is invalid"),
            Self::FirmwareLengthMismatch { expected, actual } => write!(
                formatter,
                "firmware length mismatch: manifest declares {expected} bytes, received {actual}"
            ),
            Self::FirmwareHashMismatch => {
                formatter.write_str("firmware SHA-256 digest does not match the signed manifest")
            }
            Self::TargetMismatch => {
                formatter.write_str("firmware manifest targets a different device class")
            }
            Self::RollbackDetected { current, proposed } => write!(
                formatter,
                "rollback counter must increase: current {current}, proposed {proposed}"
            ),
            Self::NoVerifiedUpdate => formatter.write_str("no verified firmware update is pending"),
            Self::LegacyApiUnsupported { replacement } => write!(
                formatter,
                "legacy OTA API is fail-closed; use {replacement}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OtaError {}

/// Canonical metadata signed by a firmware publisher.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtaManifest {
    target: String,
    version: String,
    rollback_counter: u64,
    firmware_len: u64,
    firmware_sha256: [u8; 32],
}

impl OtaManifest {
    /// Builds a manifest and binds it to the supplied firmware bytes.
    pub fn from_firmware(
        target: impl Into<String>,
        version: impl Into<String>,
        rollback_counter: u64,
        firmware: &[u8],
    ) -> Result<Self, OtaError> {
        let target = target.into();
        let version = version.into();
        validate_text_fields(&target, &version)?;
        if firmware.is_empty() {
            return Err(OtaError::EmptyFirmware);
        }
        let firmware_len = u64::try_from(firmware.len()).map_err(|_| OtaError::FirmwareTooLarge)?;

        Ok(Self {
            target,
            version,
            rollback_counter,
            firmware_len,
            firmware_sha256: Sha256::digest(firmware).into(),
        })
    }

    /// Returns the device or board class this artifact targets.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Returns the advertised firmware version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the monotonic anti-rollback counter.
    #[must_use]
    pub fn rollback_counter(&self) -> u64 {
        self.rollback_counter
    }

    /// Returns the signed firmware length.
    #[must_use]
    pub fn firmware_len(&self) -> u64 {
        self.firmware_len
    }

    /// Returns the signed SHA-256 firmware digest.
    #[must_use]
    pub fn firmware_sha256(&self) -> [u8; 32] {
        self.firmware_sha256
    }

    /// Encodes the exact domain-separated bytes that publishers must sign.
    pub fn signing_bytes(&self) -> Result<Vec<u8>, OtaError> {
        let target_len =
            u32::try_from(self.target.len()).map_err(|_| OtaError::ManifestFieldTooLong)?;
        let version_len =
            u32::try_from(self.version.len()).map_err(|_| OtaError::ManifestFieldTooLong)?;
        let mut encoded = Vec::with_capacity(
            MANIFEST_DOMAIN.len()
                + 4
                + self.target.len()
                + 4
                + self.version.len()
                + 8
                + 8
                + self.firmware_sha256.len(),
        );
        encoded.extend_from_slice(MANIFEST_DOMAIN);
        encoded.extend_from_slice(&target_len.to_be_bytes());
        encoded.extend_from_slice(self.target.as_bytes());
        encoded.extend_from_slice(&version_len.to_be_bytes());
        encoded.extend_from_slice(self.version.as_bytes());
        encoded.extend_from_slice(&self.rollback_counter.to_be_bytes());
        encoded.extend_from_slice(&self.firmware_len.to_be_bytes());
        encoded.extend_from_slice(&self.firmware_sha256);
        Ok(encoded)
    }
}

/// Result of selecting the inactive partition for an already verified update.
///
/// This receipt is not proof that platform-specific flashing or reboot succeeded.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtaCommit {
    target_partition: BootPartition,
    version: String,
    rollback_counter: u64,
}

impl OtaCommit {
    #[must_use]
    pub fn target_partition(&self) -> BootPartition {
        self.target_partition
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn rollback_counter(&self) -> u64 {
        self.rollback_counter
    }
}

/// Fail-closed verifier and boot-selection state machine for signed firmware.
#[non_exhaustive]
pub struct OtaManager {
    pub current_partition: BootPartition,
    pub status: OtaStatus,
    pub firmware_version: String,
    target: String,
    rollback_counter: u64,
    trusted_key: VerifyingKey,
    pending_manifest: Option<OtaManifest>,
}

impl OtaManager {
    /// Legacy construction without a trusted key is permanently fail-closed.
    #[deprecated(
        since = "12.0.0",
        note = "use OtaManager::new_with_trusted_key with a provisioned public key"
    )]
    pub fn new(_version: impl Into<String>) -> Result<Self, OtaError> {
        Err(OtaError::LegacyApiUnsupported {
            replacement: "OtaManager::new_with_trusted_key",
        })
    }

    /// Creates a verifier from a provisioned Ed25519 public key and the last
    /// durably committed rollback counter.
    pub fn new_with_trusted_key(
        target: impl Into<String>,
        current_version: impl Into<String>,
        rollback_counter: u64,
        trusted_public_key: [u8; 32],
    ) -> Result<Self, OtaError> {
        let target = target.into();
        let current_version = current_version.into();
        validate_text_fields(&target, &current_version)?;
        let trusted_key = VerifyingKey::from_bytes(&trusted_public_key)
            .map_err(|_| OtaError::InvalidTrustedKey)?;
        if trusted_key.is_weak() {
            return Err(OtaError::InvalidTrustedKey);
        }

        Ok(Self {
            current_partition: BootPartition::PartitionA,
            status: OtaStatus::Idle,
            firmware_version: current_version,
            target,
            rollback_counter,
            trusted_key,
            pending_manifest: None,
        })
    }

    /// Returns the last committed monotonic counter.
    #[must_use]
    pub fn rollback_counter(&self) -> u64 {
        self.rollback_counter
    }

    /// Returns the pending manifest only after successful verification.
    #[must_use]
    pub fn pending_manifest(&self) -> Option<&OtaManifest> {
        self.pending_manifest.as_ref()
    }

    /// Verifies the target, rollback counter, firmware hash/length, and strict
    /// Ed25519 signature before making an update eligible for commit.
    pub fn verify_update(
        &mut self,
        manifest: &OtaManifest,
        firmware: &[u8],
        signature: &[u8],
    ) -> Result<(), OtaError> {
        self.status = OtaStatus::Verifying;
        self.pending_manifest = None;

        let result = self.verify_update_inner(manifest, firmware, signature);
        match result {
            Ok(()) => {
                self.pending_manifest = Some(manifest.clone());
                self.status = OtaStatus::Verified;
                Ok(())
            }
            Err(error) => {
                self.status = OtaStatus::Failed;
                Err(error)
            }
        }
    }

    fn verify_update_inner(
        &self,
        manifest: &OtaManifest,
        firmware: &[u8],
        signature: &[u8],
    ) -> Result<(), OtaError> {
        if manifest.target != self.target {
            return Err(OtaError::TargetMismatch);
        }
        if manifest.rollback_counter <= self.rollback_counter {
            return Err(OtaError::RollbackDetected {
                current: self.rollback_counter,
                proposed: manifest.rollback_counter,
            });
        }
        let actual_len = u64::try_from(firmware.len()).map_err(|_| OtaError::FirmwareTooLarge)?;
        if manifest.firmware_len != actual_len {
            return Err(OtaError::FirmwareLengthMismatch {
                expected: manifest.firmware_len,
                actual: actual_len,
            });
        }
        let actual_hash: [u8; 32] = Sha256::digest(firmware).into();
        if manifest.firmware_sha256 != actual_hash {
            return Err(OtaError::FirmwareHashMismatch);
        }
        let signature =
            Signature::from_slice(signature).map_err(|_| OtaError::InvalidSignatureEncoding)?;
        let signing_bytes = manifest.signing_bytes()?;
        self.trusted_key
            .verify_strict(&signing_bytes, &signature)
            .map_err(|_| OtaError::SignatureInvalid)
    }

    /// Selects the inactive partition for the verified manifest.
    ///
    /// Platform code remains responsible for flashing and validating that bank,
    /// persisting the rollback counter, and changing the bootloader selection.
    pub fn commit_verified_update(&mut self) -> Result<OtaCommit, OtaError> {
        let manifest = self
            .pending_manifest
            .take()
            .ok_or(OtaError::NoVerifiedUpdate)?;
        self.status = OtaStatus::Committing;
        let target_partition = self.current_partition.opposite();
        self.current_partition = target_partition;
        self.firmware_version.clone_from(&manifest.version);
        self.rollback_counter = manifest.rollback_counter;
        self.status = OtaStatus::Idle;

        Ok(OtaCommit {
            target_partition,
            version: manifest.version,
            rollback_counter: manifest.rollback_counter,
        })
    }

    /// Legacy payload-only verification cannot bind target, hash, or anti-rollback
    /// metadata and is therefore permanently fail-closed.
    #[deprecated(since = "12.0.0", note = "use OtaManager::verify_update")]
    pub fn verify_signature(&self, _payload: &[u8], _signature: &[u8]) -> Result<(), OtaError> {
        Err(OtaError::LegacyApiUnsupported {
            replacement: "OtaManager::verify_update",
        })
    }

    /// Legacy unconditional commits are permanently fail-closed.
    #[deprecated(since = "12.0.0", note = "use OtaManager::commit_verified_update")]
    pub fn commit_update(
        &mut self,
        _new_version: impl Into<String>,
    ) -> Result<OtaCommit, OtaError> {
        self.status = OtaStatus::Failed;
        self.pending_manifest = None;
        Err(OtaError::LegacyApiUnsupported {
            replacement: "OtaManager::commit_verified_update",
        })
    }
}

fn validate_text_fields(target: &str, version: &str) -> Result<(), OtaError> {
    if target.is_empty() {
        return Err(OtaError::EmptyTarget);
    }
    if version.is_empty() {
        return Err(OtaError::EmptyVersion);
    }
    u32::try_from(target.len()).map_err(|_| OtaError::ManifestFieldTooLong)?;
    u32::try_from(version.len()).map_err(|_| OtaError::ManifestFieldTooLong)?;
    Ok(())
}

#[cfg(test)]
#[path = "ota_tests.rs"]
mod tests;
