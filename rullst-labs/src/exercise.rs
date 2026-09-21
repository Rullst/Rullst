use crate::{ContentHash, ExecutionLimits, LabError, MAX_CASES, Reference, Scope};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Instructor-owned exact grader input. Never send expected answers to workers
/// or students; worker outputs are evaluated on the trusted side.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraderCase {
    pub id: Reference,
    pub input: [i64; 2],
    pub expected: i64,
}
impl std::fmt::Debug for GraderCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraderCase")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Immutable instructor-defined exercise revision. Any changed case, limit or
/// scope changes its digest; a submission cannot replace the current revision.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ExerciseWire", into = "ExerciseWire")]
pub struct Exercise {
    scope: Scope,
    id: Reference,
    revision: Reference,
    cases: Vec<GraderCase>,
    limits: ExecutionLimits,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExerciseWire {
    scope: Scope,
    id: Reference,
    revision: Reference,
    cases: Vec<GraderCase>,
    limits: ExecutionLimits,
}
impl Exercise {
    pub fn new(
        scope: Scope,
        id: Reference,
        revision: Reference,
        cases: Vec<GraderCase>,
        limits: ExecutionLimits,
    ) -> Result<Self, LabError> {
        if cases.is_empty() || cases.len() > MAX_CASES {
            return Err(LabError::InvalidInput);
        }
        let mut ids = BTreeSet::new();
        if cases.iter().any(|case| !ids.insert(&case.id)) {
            return Err(LabError::InvalidInput);
        }
        Ok(Self {
            scope,
            id,
            revision,
            cases,
            limits,
        })
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn id(&self) -> &Reference {
        &self.id
    }
    pub fn revision(&self) -> &Reference {
        &self.revision
    }
    pub fn limits(&self) -> &ExecutionLimits {
        &self.limits
    }
    /// Trusted grading only; never expose this value on student/status routes.
    pub fn grader_cases(&self) -> &[GraderCase] {
        &self.cases
    }
    pub fn digest(&self) -> Result<ContentHash, LabError> {
        let bytes = zeroize::Zeroizing::new(
            serde_json::to_vec(&(
                crate::PROTOCOL_VERSION,
                crate::PROFILE,
                crate::TOOLCHAIN,
                crate::INTERPRETER,
                self,
            ))
            .map_err(|_| LabError::InvalidInput)?,
        );
        Ok(ContentHash::of(&bytes))
    }
}
impl std::fmt::Debug for Exercise {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Exercise")
            .field("scope", &self.scope)
            .field("id", &self.id)
            .field("revision", &self.revision)
            .field("case_count", &self.cases.len())
            .finish_non_exhaustive()
    }
}
impl TryFrom<ExerciseWire> for Exercise {
    type Error = LabError;
    fn try_from(value: ExerciseWire) -> Result<Self, LabError> {
        Self::new(
            value.scope,
            value.id,
            value.revision,
            value.cases,
            value.limits,
        )
    }
}
impl From<Exercise> for ExerciseWire {
    fn from(value: Exercise) -> Self {
        Self {
            scope: value.scope,
            id: value.id,
            revision: value.revision,
            cases: value.cases,
            limits: value.limits,
        }
    }
}
