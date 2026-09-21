use crate::{LabError, Reference, Scope};
use std::future::Future;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Current course membership before looking up a job; does not grant access
    /// to another learner's status, source or result.
    AccessCourse,
    ManageExercises,
    Submit,
    ReadOwn,
    CancelOwn,
    ManageJobs,
}
/// Refresh current membership and action permission from the authoritative host.
pub trait Authorization: Send + Sync {
    fn check(
        &self,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> impl Future<Output = Result<Permission, LabError>> + Send;
}
#[derive(Debug, Clone, Copy)]
pub struct Permission {
    expires_at: i64,
}
impl Permission {
    pub fn until(expires_at: i64) -> Result<Self, LabError> {
        Ok(Self {
            expires_at: checked_time(expires_at)?,
        })
    }
    pub fn expires_at(self) -> i64 {
        self.expires_at
    }
}
pub trait Clock: Send + Sync {
    fn now(&self) -> Result<i64, LabError>;
}
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Result<i64, LabError> {
        let duration = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| LabError::Clock)?;
        checked_time(i64::try_from(duration.as_secs()).map_err(|_| LabError::Clock)?)
    }
}
pub(crate) fn checked_time(value: i64) -> Result<i64, LabError> {
    if !(1..=253_402_300_799).contains(&value) {
        return Err(LabError::Clock);
    }
    Ok(value)
}
