/// Stable minimized errors; never carries provider bodies, secrets or URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MediaError {
    #[error("invalid media input")]
    InvalidInput,
    #[error("invalid media configuration")]
    Configuration,
    #[error("media permission denied")]
    Denied,
    #[error("media asset unavailable")]
    NotFound,
    #[error("media state or revision conflict")]
    Conflict,
    #[error("media operation already in progress")]
    Busy,
    #[error("media capacity exceeded")]
    Capacity,
    #[error("trusted media clock unavailable or moved backwards")]
    Clock,
    #[error("media capability expired")]
    Expired,
    #[error("media provider temporarily unavailable")]
    Unavailable,
    #[error("media provider rejected the request")]
    Rejected,
    #[error("media provider response violated its contract")]
    Protocol,
    #[error("media operation outcome uncertain; reconcile persisted state")]
    Uncertain,
    #[error("media storage unavailable")]
    Storage,
    #[error("media signature invalid")]
    Signature,
    #[error("media capability unsupported")]
    Unsupported,
}
