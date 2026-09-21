//! Dedicated encrypted shared-local job plane. It is not an application database
//! or an execution engine. The host owns trusted local files and backup policy.
mod completion;
mod crypto;
mod exercises;
mod grading;
mod jobs;
mod leasing;
mod record;
mod recovery;
mod retention;
mod store;
mod transaction;

pub use crypto::ContentKey;
pub use grading::{CaseGrade, JobResult, ResultEvidence};
pub use leasing::{LeaseStatus, LeasedJob};
pub use record::{JobState, JobView};
pub use recovery::CleanupJob;
pub use store::{SqliteLabs, StoreConfig};
