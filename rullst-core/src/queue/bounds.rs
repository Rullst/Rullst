//! Validation of queue bounds and Unix millisecond timestamps.

use super::{MAX_STALLED_LEASES_LIMIT, QueueError};
#[cfg(any(feature = "queue-sqlite", feature = "queue-redis"))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg_attr(
    not(any(feature = "queue-sqlite", feature = "queue-redis")),
    allow(dead_code)
)]
pub(super) fn validate_max_stalled_leases(leases: u32) -> Result<u32, QueueError> {
    if (1..=MAX_STALLED_LEASES_LIMIT).contains(&leases) {
        Ok(leases)
    } else {
        Err(QueueError::InvalidConfiguration(format!(
            "the stalled-lease ceiling must be between 1 and {MAX_STALLED_LEASES_LIMIT}"
        )))
    }
}

#[cfg(any(feature = "queue-sqlite", feature = "queue-redis"))]
pub(crate) fn unix_timestamp_millis_ceil(timestamp: SystemTime) -> Result<u64, QueueError> {
    let duration = timestamp.duration_since(UNIX_EPOCH).map_err(|_| {
        QueueError::InvalidConfiguration("timestamp predates Unix epoch".to_string())
    })?;
    let whole_millis = duration.as_millis();
    let has_fractional_millisecond = duration.subsec_nanos() % 1_000_000 != 0;
    let rounded_millis = whole_millis
        .checked_add(u128::from(has_fractional_millisecond))
        .ok_or_else(|| {
            QueueError::InvalidConfiguration("timestamp exceeds queue storage range".to_string())
        })?;
    u64::try_from(rounded_millis).map_err(|_| {
        QueueError::InvalidConfiguration("timestamp exceeds queue storage range".to_string())
    })
}

#[cfg(feature = "queue-sqlite")]
pub(crate) fn unix_timestamp_millis_floor(timestamp: SystemTime) -> Result<u64, QueueError> {
    let duration = timestamp.duration_since(UNIX_EPOCH).map_err(|_| {
        QueueError::InvalidConfiguration("timestamp predates Unix epoch".to_string())
    })?;
    u64::try_from(duration.as_millis()).map_err(|_| {
        QueueError::InvalidConfiguration("timestamp exceeds queue storage range".to_string())
    })
}
