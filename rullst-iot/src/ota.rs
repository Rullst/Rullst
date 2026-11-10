//! Signed firmware manifest verification and OTA boot-selection state.
//!
//! This module verifies Ed25519 signatures over a canonical manifest that binds
//! the target, version, monotonic rollback counter, firmware length, and SHA-256
//! digest. An explicit platform store can provide durable, atomic anti-rollback
//! commits. This module does not download, flash, boot, or implement that store;
//! platform code must provide those operations and their hardware guarantees.

extern crate alloc;

use alloc::string::String;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

mod counter;
mod error;
mod manifest;
pub use counter::{RollbackCounterError, RollbackCounterStore};
pub use error::OtaError;
use manifest::validate_text_fields;
pub use manifest::{OtaCommit, OtaManifest};

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

/// Fail-closed verifier and boot-selection state machine for signed firmware.
///
/// Since 13.0.0-alpha.5 its state is read with [`Self::current_partition`],
/// [`Self::status`] and [`Self::firmware_version`]; only the verification
/// and commit methods change it.
#[non_exhaustive]
pub struct OtaManager {
    current_partition: BootPartition,
    status: OtaStatus,
    firmware_version: String,
    target: String,
    rollback_counter: u64,
    trusted_key: VerifyingKey,
    pending_manifest: Option<OtaManifest>,
    /// Manifest whose store commit failed with an outcome-unknown error.
    uncertain_commit: Option<OtaManifest>,
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
            uncertain_commit: None,
        })
    }

    /// Creates a verifier with the last committed counter loaded from a
    /// platform store.
    ///
    /// The store implementation, not `OtaManager`, is responsible for proving
    /// durable and power-loss-safe hardware semantics.
    pub fn new_with_counter_store<S: RollbackCounterStore>(
        target: impl Into<String>,
        current_version: impl Into<String>,
        trusted_public_key: [u8; 32],
        counter_store: &mut S,
    ) -> Result<Self, OtaError> {
        let rollback_counter = counter_store.load()?;
        Self::new_with_trusted_key(
            target,
            current_version,
            rollback_counter,
            trusted_public_key,
        )
    }

    /// Creates a verifier for a device running from `running_partition`, with
    /// the last committed counter loaded from a platform store.
    ///
    /// `running_partition` must be the bank the platform bootloader started,
    /// so that [`Self::verified_target_partition`] names the inactive bank.
    /// [`Self::new_with_counter_store`] instead assumes
    /// [`BootPartition::PartitionA`]. Added in 13.0.
    pub fn new_with_running_partition<S: RollbackCounterStore>(
        target: impl Into<String>,
        current_version: impl Into<String>,
        running_partition: BootPartition,
        trusted_public_key: [u8; 32],
        counter_store: &mut S,
    ) -> Result<Self, OtaError> {
        let manager = Self::new_with_counter_store(
            target,
            current_version,
            trusted_public_key,
            counter_store,
        )?;
        Ok(manager.with_running_partition(running_partition))
    }

    /// Sets the bank the device is running from, as its bootloader reports
    /// it, for a manager built with [`Self::new_with_trusted_key`] or
    /// [`Self::new_with_counter_store`], which assume
    /// [`BootPartition::PartitionA`]: for example after the device rebooted
    /// into [`BootPartition::PartitionB`]. Call it before verifying an
    /// update. An update verified or left uncommitted before the call is
    /// discarded and must be verified again, because its target bank changes.
    ///
    /// Added in 13.0.0-alpha.5; before, platform code assigned the public
    /// `current_partition` field.
    #[must_use]
    pub fn with_running_partition(mut self, running_partition: BootPartition) -> Self {
        self.current_partition = running_partition;
        self.pending_manifest = None;
        self.uncertain_commit = None;
        self.status = OtaStatus::Idle;
        self
    }

    /// The bank the device is running from. [`Self::new_with_running_partition`]
    /// and [`Self::with_running_partition`] set it; the other constructors
    /// assume [`BootPartition::PartitionA`]. A commit does not change it: the
    /// receipt names the bank selected for the next boot, which runs only
    /// after the platform reboots into it.
    ///
    /// A method since 13.0.0-alpha.5, which made the field private.
    #[must_use]
    pub fn current_partition(&self) -> BootPartition {
        self.current_partition
    }

    /// The state of the update in progress.
    ///
    /// A method since 13.0.0-alpha.5, which made the field private.
    #[must_use]
    pub fn status(&self) -> OtaStatus {
        self.status
    }

    /// The version of the firmware last committed, initially the
    /// `current_version` given to the constructor.
    ///
    /// A method since 13.0.0-alpha.5, which made the field private.
    #[must_use]
    pub fn firmware_version(&self) -> &str {
        &self.firmware_version
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

    /// Returns the inactive bank selected for a cryptographically verified
    /// update, without changing any state. This is always the bank opposite
    /// [`Self::current_partition`], never the running one, including for a
    /// further update verified before the platform reboots.
    ///
    /// Platform code can flash and read back this bank before committing the
    /// durable rollback counter.
    pub fn verified_target_partition(&self) -> Result<BootPartition, OtaError> {
        self.pending_manifest
            .as_ref()
            .ok_or(OtaError::NoVerifiedUpdate)?;
        Ok(self.current_partition.opposite())
    }

    /// Verifies the target, rollback counter, firmware hash/length, and strict
    /// Ed25519 signature before making an update eligible for commit.
    ///
    /// The checks run cheapest first: target, rollback counter, firmware
    /// length, signature encoding and the manifest's signing bytes, then the
    /// SHA-256 of `firmware` and the signature. Since 13.0.0-alpha.5 a
    /// malformed signature is reported as
    /// [`OtaError::InvalidSignatureEncoding`] before the image is hashed, so
    /// it no longer reports [`OtaError::FirmwareHashMismatch`] first.
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
        // Structural checks before the image hash, which is the expensive
        // step on a device (about 26 ms per 32 KiB on a 64 MHz Cortex-M4).
        let signature =
            Signature::from_slice(signature).map_err(|_| OtaError::InvalidSignatureEncoding)?;
        let signing_bytes = manifest.signing_bytes()?;
        let actual_hash: [u8; 32] = Sha256::digest(firmware).into();
        if manifest.firmware_sha256 != actual_hash {
            return Err(OtaError::FirmwareHashMismatch);
        }
        self.trusted_key
            .verify_strict(&signing_bytes, &signature)
            .map_err(|_| OtaError::SignatureInvalid)
    }

    /// Selects the inactive partition for the verified manifest in memory.
    ///
    /// Platform code remains responsible for flashing and validating that bank,
    /// persisting the rollback counter, and changing the bootloader selection.
    /// Prefer [`Self::commit_verified_update_with_store`] when a durable counter
    /// implementation is available.
    pub fn commit_verified_update(&mut self) -> Result<OtaCommit, OtaError> {
        let manifest = self
            .pending_manifest
            .as_ref()
            .cloned()
            .ok_or(OtaError::NoVerifiedUpdate)?;
        self.status = OtaStatus::Committing;
        Ok(self.apply_verified_manifest(manifest))
    }

    /// Atomically advances a platform rollback counter before committing the
    /// verified manifest to this manager's in-memory state.
    ///
    /// A store failure leaves the manifest verified and available for retry.
    /// A store may report a failure after it durably committed the counter, for
    /// example when an acknowledgement is lost. When a retry of that same
    /// manifest's commit then reports that the store already holds exactly its
    /// counter, and a fresh [`RollbackCounterStore::load`] confirms it, the
    /// earlier commit is completed and the receipt is returned. Any other
    /// conflict, including one for a manifest this manager never attempted,
    /// means this manager is stale and should be reconstructed from the store
    /// before another update is verified. Platform code should flash
    /// and validate [`Self::verified_target_partition`] before calling this
    /// method, then coordinate the returned receipt with its bootloader.
    pub fn commit_verified_update_with_store<S: RollbackCounterStore>(
        &mut self,
        counter_store: &mut S,
    ) -> Result<OtaCommit, OtaError> {
        let manifest = self
            .pending_manifest
            .as_ref()
            .cloned()
            .ok_or(OtaError::NoVerifiedUpdate)?;
        self.status = OtaStatus::Committing;
        if let Err(error) =
            counter_store.compare_and_set(self.rollback_counter, manifest.rollback_counter)
        {
            if self.uncertain_commit.as_ref() == Some(&manifest)
                && reports_committed_counter(error, manifest.rollback_counter)
                && counter_store.load() == Ok(manifest.rollback_counter)
            {
                return Ok(self.apply_verified_manifest(manifest));
            }
            if !matches!(
                error,
                RollbackCounterError::Conflict { .. } | RollbackCounterError::NonMonotonic { .. }
            ) {
                // The store may have committed this manifest's counter.
                self.uncertain_commit = Some(manifest);
            }
            self.status = OtaStatus::Verified;
            return Err(error.into());
        }

        Ok(self.apply_verified_manifest(manifest))
    }

    fn apply_verified_manifest(&mut self, manifest: OtaManifest) -> OtaCommit {
        // The device keeps running from `current_partition` until the platform
        // reboots into the selected bank, so a later update in this process
        // must still target the inactive bank.
        let target_partition = self.current_partition.opposite();
        self.firmware_version.clone_from(&manifest.version);
        self.rollback_counter = manifest.rollback_counter;
        self.pending_manifest = None;
        self.uncertain_commit = None;
        self.status = OtaStatus::Idle;

        OtaCommit {
            target_partition,
            version: manifest.version,
            rollback_counter: manifest.rollback_counter,
        }
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

/// Reports whether a rejected compare-and-set says the store already holds
/// `counter`, whichever of the expected-value or monotonicity checks the
/// adapter applied first.
fn reports_committed_counter(error: RollbackCounterError, counter: u64) -> bool {
    match error {
        RollbackCounterError::Conflict { actual, .. } => actual == counter,
        RollbackCounterError::NonMonotonic { current, .. } => current == counter,
        _ => false,
    }
}

#[cfg(test)]
#[path = "ota_tests.rs"]
mod tests;
