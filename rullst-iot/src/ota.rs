//! Zero-Trust Over-The-Air (OTA) Firmware Update Manager (`rullst_iot::ota`).

extern crate alloc;
use alloc::string::String;
use alloc::format;

/// Active firmware boot partition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootPartition {
    PartitionA,
    PartitionB,
}

impl BootPartition {
    /// Returns the inactive fallback partition.
    pub fn opposite(&self) -> BootPartition {
        match self {
            BootPartition::PartitionA => BootPartition::PartitionB,
            BootPartition::PartitionB => BootPartition::PartitionA,
        }
    }
}

/// OTA update session state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OtaStatus {
    Idle,
    Downloading,
    Verifying,
    Committing,
    Failed,
}

/// Zero-Trust OTA Firmware Update Manager with dual A/B partition rollback support.
pub struct OtaManager {
    pub current_partition: BootPartition,
    pub status: OtaStatus,
    pub firmware_version: String,
}

impl OtaManager {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            current_partition: BootPartition::PartitionA,
            status: OtaStatus::Idle,
            firmware_version: version.into(),
        }
    }

    /// Verifies an Ed25519 signature stub for incoming firmware payload.
    /// Production: wire against `ed25519-dalek` or `micro-ecc`.
    pub fn verify_signature(&self, payload: &[u8], signature: &[u8]) -> bool {
        // Stub: In production, use ed25519-dalek::verify
        !payload.is_empty() && signature.len() == 64
    }

    /// Commits the update to the inactive partition and schedules reboot.
    pub fn commit_update(&mut self, new_version: impl Into<String>) -> String {
        let target = self.current_partition.opposite();
        self.current_partition = target;
        self.firmware_version = new_version.into();
        self.status = OtaStatus::Idle;
        format!(
            "✅ OTA committed to {:?}. Rebooting into v{}...",
            self.current_partition, self.firmware_version
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ota_partition_swap() {
        let mut ota = OtaManager::new("12.0.0");
        assert_eq!(ota.current_partition, BootPartition::PartitionA);
        let msg = ota.commit_update("12.1.0");
        assert_eq!(ota.current_partition, BootPartition::PartitionB);
        assert!(msg.contains("12.1.0"));
    }

    #[test]
    fn test_ota_signature_verification() {
        let ota = OtaManager::new("12.0.0");
        assert!(ota.verify_signature(b"firmware_payload", &[0u8; 64]));
        assert!(!ota.verify_signature(b"", &[0u8; 64]));
    }
}
