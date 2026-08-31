//! Bounded offline synchronization foundations for native platform clients.
//!
//! The server remains authoritative. This module never treats client time,
//! cached roles, tenant claims, or locally edited revisions as authorization.
//! Applications must persist idempotency decisions and enforce domain policy on
//! the server. Platform Keychain/Keystore integration is intentionally outside
//! this crate; callers provide high-entropy key material and own its lifecycle.

mod conflict;
mod crypto;
mod error;
mod json_limit;
mod policy;
mod snapshot;
mod state;
mod types;
mod validation;

pub use conflict::{
    ConflictResolution, ErasureSummary, OfflineConflict, OfflineConflictReason, RecoverySummary,
};
pub use crypto::OfflineSnapshotCipher;
pub use error::OfflineSyncError;
pub use policy::OfflineSyncPolicy;
pub use state::OfflineSyncState;
pub use types::{
    MutationOutcome, OfflineAccountId, OfflineEntityKey, OfflineMutation, OfflineMutationKind,
    OfflineRecordValue, ServerRecord, SyncCursor, SyncPullPage, SyncPushBatch, SyncPushResult,
};

/// Current encrypted snapshot schema understood by this release.
pub const OFFLINE_SNAPSHOT_VERSION: u16 = 1;
/// Domain marker authenticated into every encrypted snapshot.
pub const OFFLINE_SNAPSHOT_DOMAIN: &str = "rullst.offline";

#[cfg(test)]
mod tests;
