//! Injectable clocks keep lease and retry contracts deterministic in tests.

use crate::{MessagingError, Result};
use std::time::{SystemTime, UNIX_EPOCH};

/// Supplies trusted broker time as non-negative Unix milliseconds.
pub trait Clock: Clone + Send + Sync + 'static {
    /// Returns the current non-negative Unix timestamp in milliseconds.
    fn now_millis(&self) -> Result<i64>;
}

/// Production clock backed by [`SystemTime`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> Result<i64> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| MessagingError::ClockOutOfRange)?;
        i64::try_from(elapsed.as_millis()).map_err(|_| MessagingError::ClockOutOfRange)
    }
}
