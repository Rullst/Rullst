use thiserror::Error;

/// Fail-closed offline-state, synchronization, quota, or cryptographic error.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OfflineSyncError {
    /// Account identifiers must be bounded portable tokens.
    #[error("offline account id must be a 1-128 byte ASCII token")]
    InvalidAccountId,
    /// Collection identifiers must be bounded portable tokens.
    #[error("offline collection must be a 1-64 byte ASCII token")]
    InvalidCollection,
    /// Entity identifiers must be bounded portable tokens.
    #[error("offline entity id must be a 1-128 byte ASCII token")]
    InvalidEntityId,
    /// Opaque synchronization cursors have a conservative wire limit.
    #[error("offline sync cursor must be a 1-256 byte ASCII token")]
    InvalidCursor,
    /// Policy values must be non-zero and remain below hard safety ceilings.
    #[error("offline sync policy is outside the supported safety bounds")]
    InvalidPolicy,
    /// A record or mutation payload exceeded the configured bound.
    #[error("offline JSON payload exceeds the {maximum}-byte maximum")]
    PayloadTooLarge {
        /// Configured payload ceiling.
        maximum: usize,
    },
    /// The pending mutation queue reached its configured quota.
    #[error("offline mutation queue reached its {maximum}-item maximum")]
    PendingQuotaExceeded {
        /// Configured pending-item ceiling.
        maximum: usize,
    },
    /// Cached server state reached its configured quota.
    #[error("offline record cache reached its {maximum}-item maximum")]
    RecordQuotaExceeded {
        /// Configured record ceiling.
        maximum: usize,
    },
    /// Unresolved conflicts reached their configured quota.
    #[error("offline conflict queue reached its {maximum}-item maximum")]
    ConflictQuotaExceeded {
        /// Configured conflict ceiling.
        maximum: usize,
    },
    /// Replay keys must be unique across pending and conflicted operations.
    #[error("offline mutation idempotency key is already present")]
    DuplicateIdempotencyKey,
    /// A server outcome did not correspond to a queued operation.
    #[error("offline server outcome references an unknown pending mutation")]
    UnknownMutation,
    /// A push response repeated an outcome for the same replay key.
    #[error("offline push response repeats an idempotency key")]
    DuplicateOutcome,
    /// A server page repeated an entity key ambiguously.
    #[error("offline server page repeats an entity key")]
    DuplicateServerRecord,
    /// Push batches must remain within configured and hard limits.
    #[error("offline push batch limit is invalid")]
    InvalidBatchLimit,
    /// A server record did not correspond to the mutation it resolved.
    #[error("offline server record key does not match its mutation")]
    RecordKeyMismatch,
    /// Server revisions begin at one.
    #[error("offline server record revision must be positive")]
    InvalidRevision,
    /// Older server revisions cannot replace newer cached authority.
    #[error("offline server record revision regressed")]
    RevisionRegressed,
    /// Equal revisions must describe exactly the same authoritative value.
    #[error("offline server reused a revision for different data")]
    RevisionCollision,
    /// Server-authored time cannot move backwards within one state lineage.
    #[error("offline server time regressed")]
    ServerTimeRegressed,
    /// Incremental state must pause until an explicit full resynchronization.
    #[error("offline server requires a full resynchronization")]
    FullResyncRequired,
    /// Conflict resolution referenced an unknown replay key.
    #[error("offline conflict was not found")]
    UnknownConflict,
    /// Snapshot bytes did not have a supported authenticated envelope.
    #[error("offline snapshot envelope is invalid")]
    InvalidSnapshot,
    /// Snapshot schema cannot be silently guessed or downgraded.
    #[error("offline snapshot version {0} is unsupported")]
    UnsupportedSnapshotVersion(u16),
    /// Encoded or encrypted snapshot exceeded the configured ceiling.
    #[error("offline snapshot exceeds the {maximum}-byte maximum")]
    SnapshotTooLarge {
        /// Configured snapshot ceiling.
        maximum: usize,
    },
    /// AES-256 requires exactly 32 bytes of high-entropy key material.
    #[error("offline snapshot key must contain exactly 32 bytes")]
    InvalidKeyLength,
    /// Rotation identifiers must use the bounded portable token grammar.
    #[error("offline snapshot key id must be a 1-64 byte ASCII token")]
    InvalidKeyId,
    /// The encrypted envelope selects another rotation key.
    #[error("offline snapshot requires a different key id")]
    SnapshotKeyIdMismatch,
    /// The operating system could not provide a secure nonce.
    #[error("operating-system randomness is unavailable")]
    RandomnessUnavailable,
    /// Authenticated encryption failed without exposing sensitive details.
    #[error("offline snapshot encryption failed")]
    EncryptionFailed,
    /// Key, account binding, nonce, tag, or ciphertext authentication failed.
    #[error("offline snapshot authentication failed")]
    AuthenticationFailed,
    /// Decrypted state belonged to another account.
    #[error("offline snapshot account binding does not match")]
    AccountMismatch,
    /// Snapshot JSON was malformed or violated its closed schema.
    #[error("offline snapshot JSON is invalid")]
    InvalidJson(#[source] serde_json::Error),
    /// Snapshot JSON encoding failed.
    #[error("offline snapshot JSON encoding failed")]
    EncodeJson(#[source] serde_json::Error),
}
