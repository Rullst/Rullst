use super::grading::{JobResult, ResultEvidence};
use crate::{
    ContentHash, Exercise, ExerciseRef, LabError as Error, Reference, RustSource, Scope,
    authorization::checked_time,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    Queued,
    Running,
    Cancelled,
    Completed,
    Simulated,
    Failed,
    Expired,
    Uncertain,
}
impl JobState {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Running => "Running",
            Self::Cancelled => "Cancelled",
            Self::Completed => "Completed",
            Self::Simulated => "Simulated",
            Self::Failed => "Failed",
            Self::Expired => "Expired",
            Self::Uncertain => "Uncertain",
        }
    }
}
/// Authorized status projection; no source, hidden tests or runner secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobView {
    pub id: Reference,
    pub scope: Scope,
    pub learner: Reference,
    pub exercise: ExerciseRef,
    pub source_digest: ContentHash,
    pub exercise_digest: ContentHash,
    pub state: JobState,
    pub revision: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub expires_at: i64,
    /// Cancellation/expiry was recorded but an owned worker still needs teardown.
    pub cleanup_pending: bool,
    pub result: Option<JobResult>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Lease {
    pub nonce: Reference,
    pub until: i64,
    pub revision: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub view: JobView,
    pub request_digest: ContentHash,
    pub profile_digest: ContentHash,
    pub lease: Option<Lease>,
    pub attempts: u8,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Payload {
    pub exercise: Exercise,
    pub source: RustSource,
}
impl Record {
    pub fn validate(&self, has_content: bool) -> Result<(), Error> {
        let v = &self.view;
        checked_time(v.created_at).map_err(|_| Error::Integrity)?;
        checked_time(v.updated_at).map_err(|_| Error::Integrity)?;
        checked_time(v.expires_at).map_err(|_| Error::Integrity)?;
        if v.revision <= 0
            || self.attempts > 2
            || (self.lease.is_some() && self.attempts == 0)
            || (v.result.is_some()
                != matches!(
                    v.state,
                    JobState::Completed | JobState::Failed | JobState::Simulated
                ))
            || (v.result.is_some() && (self.lease.is_some() || has_content))
            || v.updated_at < v.created_at
            || v.expires_at <= v.created_at
            || v.expires_at - v.created_at > 86_400
            || (matches!(v.state, JobState::Queued | JobState::Running) && !has_content)
            || (v.state == JobState::Queued && self.lease.is_some())
            || (v.state == JobState::Running && self.lease.is_none())
            || v.cleanup_pending != (self.lease.is_some() && v.state != JobState::Running)
        {
            return Err(Error::Integrity);
        }
        if let Some(result) = &v.result {
            result.validate()?;
            if (v.state == JobState::Simulated)
                != matches!(result.evidence(), ResultEvidence::Simulation)
            {
                return Err(Error::Integrity);
            }
            if (v.state == JobState::Completed) && !matches!(result, JobResult::Graded { .. }) {
                return Err(Error::Integrity);
            }
            if (v.state == JobState::Failed) && !matches!(result, JobResult::Rejected { .. }) {
                return Err(Error::Integrity);
            }
        }
        if let Some(lease) = &self.lease {
            checked_time(lease.until).map_err(|_| Error::Integrity)?;
            if lease.revision <= 0 || lease.revision > v.revision || lease.until > v.expires_at {
                return Err(Error::Integrity);
            }
        }
        Ok(())
    }
    pub fn next(&mut self, now: i64) -> Result<(), Error> {
        self.view.revision = self.view.revision.checked_add(1).ok_or(Error::Capacity)?;
        self.view.updated_at = now;
        self.view.cleanup_pending = self.lease.is_some() && self.view.state != JobState::Running;
        Ok(())
    }
}
