//! Incremental verification of a firmware image that arrives in chunks.

use super::{OtaError, OtaManager, OtaManifest, OtaStatus};
use sha2::{Digest, Sha256};

/// Verifies one firmware image as it arrives, without holding it in memory.
///
/// [`OtaManager::begin_update`] returns it after the manifest's target,
/// rollback counter and Ed25519 signature have passed, so no byte of an
/// unauthenticated image is hashed. Feed the image to [`Self::update`] in
/// chunks of any size, as they come from the network or from flash, then call
/// [`Self::finish`], which checks the length and the SHA-256 digest and makes
/// the update eligible for commit. The image is hashed once.
///
/// It holds the manager mutably until it is finished or dropped; while it
/// exists the manager's status is [`OtaStatus::Downloading`]. A failed chunk
/// or a drop before `finish` leaves the manager [`OtaStatus::Failed`] with no
/// pending update. Added in 13.0.0-alpha.5.
#[non_exhaustive]
#[must_use = "an update is not verified until `finish` succeeds"]
pub struct OtaImageVerifier<'a> {
    pub(super) manager: &'a mut OtaManager,
    /// The manifest whose signature passed; `finish` moves it to the manager.
    manifest: OtaManifest,
    hasher: Sha256,
    received: u64,
    /// The first error; every later call returns it again.
    failure: Option<OtaError>,
    finished: bool,
}

impl<'a> OtaImageVerifier<'a> {
    pub(super) fn new(manager: &'a mut OtaManager, manifest: OtaManifest) -> Self {
        manager.status = OtaStatus::Downloading;
        Self {
            manager,
            manifest,
            hasher: Sha256::new(),
            received: 0,
            failure: None,
            finished: false,
        }
    }

    /// The manifest being verified.
    #[must_use]
    pub fn manifest(&self) -> &OtaManifest {
        &self.manifest
    }

    /// The number of image bytes accepted so far.
    #[must_use]
    pub fn received(&self) -> u64 {
        self.received
    }

    /// The number of image bytes the manifest still expects.
    #[must_use]
    pub fn remaining(&self) -> u64 {
        self.expected_len().saturating_sub(self.received)
    }

    /// Hashes the next chunk of the image. A chunk that would take the image
    /// past the signed length is refused before it is hashed, with
    /// [`OtaError::FirmwareLengthMismatch`]; after any error the verifier
    /// stays failed and returns that error again.
    pub fn update(&mut self, chunk: &[u8]) -> Result<(), OtaError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let total = u64::try_from(chunk.len())
            .ok()
            .and_then(|len| self.received.checked_add(len))
            .ok_or(OtaError::FirmwareTooLarge);
        let total = match total {
            Ok(total) => total,
            Err(error) => return Err(self.fail(error)),
        };
        let expected = self.expected_len();
        if total > expected {
            return Err(self.fail(OtaError::FirmwareLengthMismatch {
                expected,
                actual: total,
            }));
        }
        self.hasher.update(chunk);
        self.received = total;
        Ok(())
    }

    /// Checks the image's length and SHA-256 digest against the signed
    /// manifest. On success the manager is [`OtaStatus::Verified`] and
    /// [`OtaManager::verified_target_partition`] and the commit methods apply
    /// to this update.
    pub fn finish(mut self) -> Result<(), OtaError> {
        self.finished = true;
        let result = self.check();
        match result {
            Ok(()) => {
                // An empty manifest, which allocates nothing, takes its place.
                let manifest = core::mem::replace(&mut self.manifest, OtaManifest::empty());
                self.manager.pending_manifest = Some(manifest);
                self.manager.status = OtaStatus::Verified;
            }
            Err(_) => self.manager.status = OtaStatus::Failed,
        }
        result
    }

    fn check(&mut self) -> Result<(), OtaError> {
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        let expected = self.expected_len();
        if self.received != expected {
            return Err(OtaError::FirmwareLengthMismatch {
                expected,
                actual: self.received,
            });
        }
        let digest: [u8; 32] = core::mem::take(&mut self.hasher).finalize().into();
        if self.manifest.firmware_sha256 != digest {
            return Err(OtaError::FirmwareHashMismatch);
        }
        Ok(())
    }

    fn expected_len(&self) -> u64 {
        self.manifest.firmware_len
    }

    fn fail(&mut self, error: OtaError) -> OtaError {
        self.failure = Some(error.clone());
        self.manager.status = OtaStatus::Failed;
        error
    }
}

impl Drop for OtaImageVerifier<'_> {
    fn drop(&mut self) {
        if !self.finished {
            self.manager.status = OtaStatus::Failed;
        }
    }
}
