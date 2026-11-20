use crate::{ContentHash, LabError};
use serde::{Deserialize, Serialize};

/// Naming a mode does not prove that its isolation requirements are enforced.
///
/// `#[non_exhaustive]`: this enum is the extension point for another language
/// or a native profile, which needs a new protocol version with its own tool
/// slots, source and case types. The protocol v1 types (`ToolIdentity`,
/// `RustSource`, `WorkerInput`) keep their names and wire form, because they
/// are bound into every stored profile and exercise digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub enum ExecutionProfile {
    /// Explicit simulation, never a real grade/execution attestation.
    Simulation,
    /// A separately deployed runner whose tools and receipt key the operator
    /// pins; actual per-job hardening is still mandatory. The only profile
    /// whose signed receipts `complete` and `reconcile_cleanup` accept. This
    /// crate verifies neither the platform, the pinned tools nor any
    /// isolation; the host-owned runner must enforce them.
    ///
    /// Named `LinuxExperimental` before 13.0.0-rc.1. Its serialized name,
    /// bound into every stored profile and its digest, is still
    /// `LinuxExperimental`, so stores, jobs and receipts written before the
    /// rename keep opening and verifying.
    #[serde(rename = "LinuxExperimental")]
    PinnedRunner {
        tools: ToolIdentity,
        /// Pinned Ed25519 public key; the private seed belongs only to the
        /// separately deployed trusted runner controller.
        receipt_key: ContentHash,
    },
}

/// Pin independently verified files; caller-supplied hashes are configuration,
/// not evidence that a worker actually loaded or enforced those tools.
///
/// Exhaustive on purpose: these are the seven slots of the protocol v1
/// profile. Their names follow the removed Rust-to-Wasm candidate and are
/// serialized into the store binding, so they stay as they are: a runner
/// records what fills each slot for its own tools, and another language gets
/// its own `ExecutionProfile` variant instead of new fields here.
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

#[cfg(test)]
#[path = "profile_tests.rs"]
pub(crate) mod tests;
