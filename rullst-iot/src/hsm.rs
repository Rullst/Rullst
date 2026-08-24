//! Deterministic HSM-shaped fixtures for tests and demos.
//!
//! This module is available only with `experimental-simulators`. It does not
//! communicate with hardware, protect keys, or create digital signatures.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use sha2::{Digest, Sha256};

/// Label used to generate deterministic, non-secret hardware fixture data.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulatedHsmProfile {
    Atecc608A,
    Tpm2,
    Stsafe,
    Software,
}

/// Deterministic fixture generator; not a Hardware Security Module.
#[non_exhaustive]
pub struct SimulatedHsmDevice {
    profile: SimulatedHsmProfile,
    serial: String,
}

impl SimulatedHsmDevice {
    /// Creates an explicitly simulated device fixture.
    #[must_use]
    pub fn new(profile: SimulatedHsmProfile, serial: impl Into<String>) -> Self {
        Self {
            profile,
            serial: serial.into(),
        }
    }

    #[must_use]
    pub fn profile(&self) -> SimulatedHsmProfile {
        self.profile
    }

    #[must_use]
    pub fn serial(&self) -> &str {
        &self.serial
    }

    /// Derives deterministic fixture bytes. They are public and not key material.
    #[must_use]
    pub fn derive_fixture_bytes(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"RULLST-SIMULATED-HSM-FIXTURE-V1\0");
        hasher.update(self.serial.as_bytes());
        hasher.update(format!("{:?}", self.profile).as_bytes());
        hasher.finalize().into()
    }

    /// Produces a deterministic digest fixture. It is not a signature or MAC.
    #[must_use]
    pub fn digest_fixture(&self, payload: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"RULLST-SIMULATED-HSM-DIGEST-V1\0");
        hasher.update(self.derive_fixture_bytes());
        hasher.update(payload);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulated_hsm_fixtures_are_deterministic_and_explicit() {
        let device = SimulatedHsmDevice::new(SimulatedHsmProfile::Software, "fixture-device-1");
        assert_eq!(device.derive_fixture_bytes(), device.derive_fixture_bytes());
        assert_ne!(
            device.digest_fixture(b"payload-a"),
            device.digest_fixture(b"payload-b")
        );
    }
}
