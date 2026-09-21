/// Minimized typed failures. Never includes learner source, expected answers,
/// secret material, filesystem paths or raw process/provider output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LabError {
    #[error("invalid lab input")]
    InvalidInput,
    #[error("lab permission denied")]
    Denied,
    #[error("invalid lab configuration")]
    Configuration,
    #[error("lab job or exercise unavailable")]
    NotFound,
    #[error("lab identity or revision conflict")]
    Conflict,
    #[error("lab work already leased")]
    Busy,
    #[error("lab capacity exceeded")]
    Capacity,
    #[error("trusted lab clock unavailable or moved backwards")]
    Clock,
    #[error("lab permission or execution expired")]
    Expired,
    #[error("lab storage unavailable")]
    Storage,
    #[error("lab content or receipt integrity failed")]
    Integrity,
    #[error("lab execution profile unsupported or unenforced")]
    Unsupported,
    #[error("lab execution outcome uncertain; reconcile persisted work")]
    Uncertain,
    #[error("lab worker response violated its contract")]
    Protocol,
}
