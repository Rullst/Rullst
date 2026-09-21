use crate::{LabError, Reference, RustSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExerciseRef {
    pub id: Reference,
    pub revision: Reference,
}
impl ExerciseRef {
    pub fn new(id: impl Into<String>, revision: impl Into<String>) -> Result<Self, LabError> {
        Ok(Self {
            id: Reference::new(id)?,
            revision: Reference::new(revision)?,
        })
    }
}

/// Client submission contains no tenant, actor, expected answers or executable
/// command. The server supplies authenticated scope and the registered grader.
#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "SubmissionWire", into = "SubmissionWire")]
pub struct Submission {
    pub(crate) id: Reference,
    pub(crate) exercise: ExerciseRef,
    pub(crate) source: RustSource,
    pub(crate) ttl_seconds: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SubmissionWire {
    id: Reference,
    exercise: ExerciseRef,
    source: RustSource,
    ttl_seconds: u32,
}
impl Submission {
    pub fn new(
        id: Reference,
        exercise: ExerciseRef,
        source: RustSource,
        ttl_seconds: u32,
    ) -> Result<Self, LabError> {
        if !(1..=86_400).contains(&ttl_seconds) {
            return Err(LabError::InvalidInput);
        }
        Ok(Self {
            id,
            exercise,
            source,
            ttl_seconds,
        })
    }
    pub fn id(&self) -> &Reference {
        &self.id
    }
    pub fn exercise(&self) -> &ExerciseRef {
        &self.exercise
    }
}
impl std::fmt::Debug for Submission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Submission")
            .field("id", &self.id)
            .field("exercise", &self.exercise)
            .finish_non_exhaustive()
    }
}
impl TryFrom<SubmissionWire> for Submission {
    type Error = LabError;
    fn try_from(value: SubmissionWire) -> Result<Self, LabError> {
        Self::new(value.id, value.exercise, value.source, value.ttl_seconds)
    }
}
impl From<Submission> for SubmissionWire {
    fn from(value: Submission) -> Self {
        Self {
            id: value.id,
            exercise: value.exercise,
            source: value.source,
            ttl_seconds: value.ttl_seconds,
        }
    }
}
