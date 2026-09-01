use core::fmt;

/// Failures exposed by a platform rollback-counter implementation.
///
/// Implementations should map device-specific diagnostics to these bounded
/// variants rather than exposing secrets or raw storage contents.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollbackCounterError {
    /// The counter could not be accessed without proving corruption.
    Unavailable,
    /// The stored state failed the platform's integrity checks.
    CorruptState,
    /// Another writer changed the counter after the caller loaded it.
    Conflict { expected: u64, actual: u64 },
    /// A requested transition would keep or decrease the committed value.
    NonMonotonic { current: u64, proposed: u64 },
}

impl fmt::Display for RollbackCounterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("rollback counter storage is unavailable"),
            Self::CorruptState => formatter.write_str("rollback counter state is corrupt"),
            Self::Conflict { expected, actual } => write!(
                formatter,
                "rollback counter changed concurrently: expected {expected}, actual {actual}"
            ),
            Self::NonMonotonic { current, proposed } => write!(
                formatter,
                "rollback counter must increase: current {current}, proposed {proposed}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RollbackCounterError {}

/// Platform boundary for persistent OTA anti-rollback state.
///
/// `compare_and_set` must reject a value that is not strictly greater than the
/// currently committed value, must make no change when `expected` differs from
/// that value, and may return `Ok(())` only after `proposed` is durably committed
/// across device reset. The implementation owns flash/HSM integrity,
/// wear-leveling, and power-loss atomicity; implementing this trait is not by
/// itself evidence that those hardware guarantees exist.
pub trait RollbackCounterStore {
    /// Loads the last durably committed counter.
    fn load(&mut self) -> Result<u64, RollbackCounterError>;

    /// Atomically commits a strictly increasing counter if the current value
    /// still equals `expected`.
    fn compare_and_set(&mut self, expected: u64, proposed: u64)
    -> Result<(), RollbackCounterError>;
}
