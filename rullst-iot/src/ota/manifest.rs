//! The signed manifest of a firmware image and the receipt of a commit.

extern crate alloc;

use super::{BootPartition, OtaError};
use alloc::string::String;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

const MANIFEST_DOMAIN: &[u8] = b"RULLST-OTA-MANIFEST-V1\0";

/// Canonical metadata signed by a firmware publisher.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtaManifest {
    pub(super) target: String,
    pub(super) version: String,
    pub(super) rollback_counter: u64,
    pub(super) firmware_len: u64,
    pub(super) firmware_sha256: [u8; 32],
}

impl OtaManifest {
    /// Builds a manifest from the fields a device receives with an update,
    /// without the image: its length and SHA-256 digest come from the
    /// publisher, and [`OtaManager::begin_update`](super::OtaManager::begin_update)
    /// checks them against the image as it streams in. The signature, verified
    /// there too, is what makes the fields trustworthy. Added in 13.0.0-alpha.5.
    pub fn new(
        target: impl Into<String>,
        version: impl Into<String>,
        rollback_counter: u64,
        firmware_len: u64,
        firmware_sha256: [u8; 32],
    ) -> Result<Self, OtaError> {
        let target = target.into();
        let version = version.into();
        validate_text_fields(&target, &version)?;
        if firmware_len == 0 {
            return Err(OtaError::EmptyFirmware);
        }
        Ok(Self {
            target,
            version,
            rollback_counter,
            firmware_len,
            firmware_sha256,
        })
    }

    /// Parses the canonical bytes that [`Self::signing_bytes`] produces, so a
    /// publisher can send exactly the bytes it signed and the device verifies
    /// the signature over the same bytes. Missing or trailing bytes, another
    /// domain prefix and non-UTF-8 text are [`OtaError::MalformedManifest`];
    /// empty fields get the errors of [`Self::new`]. Added in 13.0.0-alpha.5.
    pub fn from_signing_bytes(bytes: &[u8]) -> Result<Self, OtaError> {
        let mut reader = ManifestReader(bytes);
        if reader.take(MANIFEST_DOMAIN.len())? != MANIFEST_DOMAIN {
            return Err(OtaError::MalformedManifest);
        }
        let target = reader.text()?;
        let version = reader.text()?;
        let rollback_counter = u64::from_be_bytes(reader.array()?);
        let firmware_len = u64::from_be_bytes(reader.array()?);
        let firmware_sha256 = reader.array()?;
        if !reader.0.is_empty() {
            return Err(OtaError::MalformedManifest);
        }
        Self::new(
            target,
            version,
            rollback_counter,
            firmware_len,
            firmware_sha256,
        )
    }

    /// Builds a manifest and binds it to the supplied firmware bytes.
    ///
    /// This hashes `firmware`, which suits the publisher. A device that
    /// receives the manifest fields uses [`Self::new`] or
    /// [`Self::from_signing_bytes`] instead, so that it hashes the image only
    /// once, while verifying it.
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
    ///
    /// The buffer is allocated fallibly: when the allocator refuses it, this
    /// returns [`OtaError::AllocationFailed`] instead of aborting.
    pub fn signing_bytes(&self) -> Result<Vec<u8>, OtaError> {
        let target_len =
            u32::try_from(self.target.len()).map_err(|_| OtaError::ManifestFieldTooLong)?;
        let version_len =
            u32::try_from(self.version.len()).map_err(|_| OtaError::ManifestFieldTooLong)?;
        // Domain, two u32 lengths, the two fields, two u64s and the digest.
        let length = [
            MANIFEST_DOMAIN.len(),
            4,
            self.target.len(),
            4,
            self.version.len(),
            8,
            8,
            self.firmware_sha256.len(),
        ]
        .into_iter()
        .try_fold(0usize, usize::checked_add)
        .ok_or(OtaError::ManifestFieldTooLong)?;
        let mut encoded = Vec::new();
        encoded
            .try_reserve_exact(length)
            .map_err(|_| OtaError::AllocationFailed)?;
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
    pub(super) target_partition: BootPartition,
    pub(super) version: String,
    pub(super) rollback_counter: u64,
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

impl OtaManifest {
    /// A placeholder with empty fields, which allocates nothing.
    pub(super) const fn empty() -> Self {
        Self {
            target: String::new(),
            version: String::new(),
            rollback_counter: 0,
            firmware_len: 0,
            firmware_sha256: [0; 32],
        }
    }
}

/// Reads the length-prefixed fields of [`OtaManifest::signing_bytes`].
struct ManifestReader<'a>(&'a [u8]);

impl<'a> ManifestReader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], OtaError> {
        if self.0.len() < len {
            return Err(OtaError::MalformedManifest);
        }
        let (head, rest) = self.0.split_at(len);
        self.0 = rest;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], OtaError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    /// A `u32` big-endian length and that many UTF-8 bytes, copied fallibly.
    fn text(&mut self) -> Result<String, OtaError> {
        let len = u32::from_be_bytes(self.array()?);
        let len = usize::try_from(len).map_err(|_| OtaError::MalformedManifest)?;
        let text =
            core::str::from_utf8(self.take(len)?).map_err(|_| OtaError::MalformedManifest)?;
        let mut owned = String::new();
        owned
            .try_reserve_exact(text.len())
            .map_err(|_| OtaError::AllocationFailed)?;
        owned.push_str(text);
        Ok(owned)
    }
}

pub(super) fn validate_text_fields(target: &str, version: &str) -> Result<(), OtaError> {
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
