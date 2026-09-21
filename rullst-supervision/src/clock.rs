use crate::SupervisionError as Error;

pub(crate) const MAX_TIME: i64 = 253_402_300_799;

/// Trusted server UTC seconds, never client timestamps. Implementations may be
/// injected for deterministic tests and must fail if trustworthy time is absent.
pub trait Clock: Send + Sync {
    fn now(&self) -> Result<i64, Error>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Result<i64, Error> {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::Clock)?
            .as_secs();
        let seconds = i64::try_from(seconds).map_err(|_| Error::Clock)?;
        checked_time(seconds)
    }
}

pub(crate) fn checked_time(value: i64) -> Result<i64, Error> {
    if !(0..=MAX_TIME).contains(&value) {
        return Err(Error::Clock);
    }
    Ok(value)
}
