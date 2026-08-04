//! Hardware Security Element (HSM) Bindings (`rullst_iot::hsm`).
//! 
//! Production targets: ATECC608A, TPM 2.0, STSAFE.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use sha2::{Sha256, Digest};

/// Supported hardware security chip types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HsmChipType {
    Atecc608A,
    Tpm2,
    Stsafe,
    /// Software HSM stub (for simulation / non-HSM targets).
    Software,
}

/// Hardware Security Module binding abstraction.
pub struct HsmDevice {
    pub chip: HsmChipType,
    /// Simulated device serial number.
    pub serial: String,
}

impl HsmDevice {
    /// Creates a new HSM device handle.
    pub fn new(chip: HsmChipType, serial: impl Into<String>) -> Self {
        Self {
            chip,
            serial: serial.into(),
        }
    }

    /// Derives a device-unique binding key using SHA-256 of serial + chip type.
    pub fn derive_key(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.serial.as_bytes());
        hasher.update(format!("{:?}", self.chip).as_bytes());
        hasher.finalize().to_vec()
    }

    /// Signs a payload using derived hardware key (stub: returns SHA-256 digest).
    pub fn sign(&self, payload: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.derive_key());
        hasher.update(payload);
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsm_key_derivation() {
        let hsm = HsmDevice::new(HsmChipType::Software, "SN-ABC123");
        let key = hsm.derive_key();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_hsm_sign() {
        let hsm = HsmDevice::new(HsmChipType::Software, "SN-ABC123");
        let signature = hsm.sign(b"telemetry_payload");
        assert_eq!(signature.len(), 32);
    }
}
