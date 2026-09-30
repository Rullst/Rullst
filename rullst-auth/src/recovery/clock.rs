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

/// Largest step, in seconds, by which this host's clock may trail a shared
/// high-water mark and still be treated as cross-host skew. Hosts cross each
/// whole-second boundary at slightly different instants, so an NTP-synchronized
/// host can read one second less than the time another host just recorded.
#[cfg(any(
    feature = "email-login-sqlite",
    feature = "email-login-postgres",
    feature = "api-tokens-sqlite",
    feature = "api-tokens-postgres"
))]
const MAX_CLOCK_SKEW_SECONDS: i64 = 5;

/// Advances a shared observed time monotonically. A reading within the skew
/// tolerance adopts `floor`, so shared time never moves backwards and no
/// lifetime is extended; a larger regression fails closed.
#[cfg(any(
    feature = "email-login-sqlite",
    feature = "email-login-postgres",
    feature = "api-tokens-sqlite",
    feature = "api-tokens-postgres"
))]
pub(super) fn advance_clock(local: i64, floor: i64) -> Result<i64, RecoveryError> {
    if local.saturating_add(MAX_CLOCK_SKEW_SECONDS) < floor {
        return Err(RecoveryError::InvalidAction);
    }
    Ok(local.max(floor))
}

#[cfg(all(
    test,
    any(
        feature = "email-login-sqlite",
        feature = "email-login-postgres",
        feature = "api-tokens-sqlite",
        feature = "api-tokens-postgres"
    )
))]
mod tests {
    use super::*;

    #[test]
    fn shared_clock_tolerates_bounded_skew_and_stays_monotonic() {
        assert_eq!(advance_clock(1_000, 1_000), Ok(1_000));
        assert_eq!(advance_clock(999, 1_000), Ok(1_000));
        assert_eq!(advance_clock(995, 1_000), Ok(1_000));
        assert_eq!(advance_clock(1_007, 1_000), Ok(1_007));
        assert_eq!(advance_clock(994, 1_000), Err(RecoveryError::InvalidAction));
        assert_eq!(
            advance_clock(i64::MIN, 0),
            Err(RecoveryError::InvalidAction)
        );
    }
}
