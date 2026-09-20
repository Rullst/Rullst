use super::AgeError;
use std::time::{SystemTime, UNIX_EPOCH};

/// Trusted server time in Unix seconds. Never derive it from a browser or proof.
/// Custom clocks must be synchronized across every verifier sharing a store.
pub trait AgeClock: Send + Sync {
    fn now(&self) -> Result<i64, AgeError>;
}

/// Default server clock. Invalid system time fails closed.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemAgeClock;

impl AgeClock for SystemAgeClock {
    fn now(&self) -> Result<i64, AgeError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_secs()).ok())
            .ok_or(AgeError::InvalidChallenge)
    }
}
