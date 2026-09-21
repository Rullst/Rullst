use crate::{ContentHash, LabError};
use serde::{Deserialize, Serialize};

/// Naming a mode does not prove that its isolation requirements are enforced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ExecutionProfile {
    /// Explicit simulation, never a real grade/execution attestation.
    Simulation,
    /// Operator-selected candidate; actual per-job hardening is still mandatory.
    LinuxExperimental {
        tools: ToolIdentity,
        /// Pinned Ed25519 public key; the private seed belongs only to the
        /// separately deployed trusted runner controller.
        receipt_key: ContentHash,
    },
}

/// Pin independently verified files; caller-supplied hashes are configuration,
/// not evidence that a worker actually loaded or enforced those tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolIdentity {
    pub runner: ContentHash,
    pub compiler: ContentHash,
    pub wasm_toolchain: ContentHash,
    pub runtime: ContentHash,
    pub launcher: ContentHash,
    pub syscall_policy: ContentHash,
    pub filesystem_policy: ContentHash,
}
impl ExecutionProfile {
    pub fn digest(&self) -> Result<ContentHash, LabError> {
        let bytes = serde_json::to_vec(&(
            crate::PROTOCOL_VERSION,
            crate::PROFILE,
            crate::TOOLCHAIN,
            crate::INTERPRETER,
            self,
        ))
        .map_err(|_| LabError::Configuration)?;
        Ok(ContentHash::of(&bytes))
    }
}
