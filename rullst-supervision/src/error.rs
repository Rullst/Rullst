/// Errors never include identity, evidence, database paths or submitted content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SupervisionError {
    #[error("invalid supervision input")]
    InvalidInput,
    #[error("supervision access denied")]
    Forbidden,
    #[error("supervision state changed; reload before retrying")]
    Conflict,
    #[error("supervision state expired")]
    Expired,
    #[error("supervision capacity exhausted")]
    Capacity,
    #[error("supervision event rate exceeded")]
    RateLimited,
    #[error("unexpected supervision event sequence")]
    Sequence,
    #[error("invalid supervision configuration or state")]
    Configuration,
    #[error("supervision storage unavailable")]
    Storage,
    #[error("supervision commit outcome is uncertain; reload before retrying")]
    UncertainCommit,
    #[error("trusted supervision clock unavailable or moved backwards")]
    Clock,
    #[error("supervision revision exhausted")]
    RevisionExhausted,
}
