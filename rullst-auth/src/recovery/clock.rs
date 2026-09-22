use super::RecoveryError;
use std::time::{SystemTime, UNIX_EPOCH};

/// Trusted server time, never a timestamp supplied by a request or bearer token.
pub trait AuthClock: Send + Sync {
    fn now(&self) -> Result<u64, RecoveryError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemAuthClock;
impl AuthClock for SystemAuthClock {
    fn now(&self) -> Result<u64, RecoveryError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .map_err(|_| RecoveryError::InvalidAction)
    }
}
