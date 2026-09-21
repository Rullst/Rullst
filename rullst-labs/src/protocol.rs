use crate::{ContentHash, ExecutionLimits, LabError, MAX_CASES, Reference, RustSource};
use serde::{Deserialize, Serialize};

/// Opaque bindings sent to an untrusted worker. No tenant/learner identity or
/// expected answers are needed inside the compiler/interpreter process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptBinding {
    pub request: ContentHash,
    pub profile: ContentHash,
    pub source: ContentHash,
    pub nonce: Reference,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "InputWire", into = "InputWire")]
pub struct WorkerInput {
    binding: AttemptBinding,
    source: RustSource,
    inputs: Vec<[i64; 2]>,
    limits: ExecutionLimits,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputWire {
    binding: AttemptBinding,
    source: RustSource,
    inputs: Vec<[i64; 2]>,
    limits: ExecutionLimits,
}
impl WorkerInput {
    pub fn new(
        binding: AttemptBinding,
        source: RustSource,
        inputs: Vec<[i64; 2]>,
        limits: ExecutionLimits,
    ) -> Result<Self, LabError> {
        if binding.source != source.digest() || inputs.is_empty() || inputs.len() > MAX_CASES {
            return Err(LabError::InvalidInput);
        }
        Ok(Self {
            binding,
            source,
            inputs,
            limits,
        })
    }
    pub fn binding(&self) -> &AttemptBinding {
        &self.binding
    }
    pub fn source(&self) -> &RustSource {
        &self.source
    }
    pub fn inputs(&self) -> &[[i64; 2]] {
        &self.inputs
    }
    pub fn limits(&self) -> &ExecutionLimits {
        &self.limits
    }
    /// The outer transport must bound bytes before allocating/decoding JSON.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LabError> {
        if bytes.len() > 131_072 {
            return Err(LabError::Capacity);
        }
        serde_json::from_slice(bytes).map_err(|_| LabError::Protocol)
    }
}
impl std::fmt::Debug for WorkerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerInput")
            .field("binding", &self.binding)
            .field("case_count", &self.inputs.len())
            .finish_non_exhaustive()
    }
}
impl TryFrom<InputWire> for WorkerInput {
    type Error = LabError;
    fn try_from(v: InputWire) -> Result<Self, LabError> {
        Self::new(v.binding, v.source, v.inputs, v.limits)
    }
}
impl From<WorkerInput> for InputWire {
    fn from(v: WorkerInput) -> Self {
        Self {
            binding: v.binding,
            source: v.source,
            inputs: v.inputs,
            limits: v.limits,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrapKind {
    Fuel,
    Memory,
    Stack,
    Guest,
}
/// Raw values are transient trusted-grader input, never a student-facing result.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum CaseOutput {
    Value(i64),
    Trap(TrapKind),
}
impl std::fmt::Debug for CaseOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(_) => f.write_str("Value([redacted])"),
            Self::Trap(kind) => f.debug_tuple("Trap").field(kind).finish(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionFailure {
    Compile,
    InvalidModule,
    ResourceLimit,
    WorkerProtocol,
    WorkerLost,
    Isolation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum WorkerOutcome {
    CompileRejected {
        diagnostics: crate::Diagnostic,
    },
    Executed {
        artifact: ContentHash,
        cases: Vec<CaseOutput>,
    },
    Rejected(ExecutionFailure),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerOutput {
    pub binding: AttemptBinding,
    pub outcome: WorkerOutcome,
}
impl WorkerOutput {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LabError> {
        // A valid diagnostic can double in size when JSON escapes its text.
        // Leave room for the authenticated binding, matching the wire reader.
        if bytes.len() > 20_480 {
            return Err(LabError::Capacity);
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| LabError::Protocol)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), LabError> {
        if let WorkerOutcome::Executed { cases, .. } = &self.outcome
            && (cases.is_empty() || cases.len() > MAX_CASES)
        {
            return Err(LabError::Protocol);
        }
        Ok(())
    }
}
/// Deliberate offline data. This does not parse or run source and its output
/// cannot become an execution receipt or a production grade.
pub fn simulate(input: &WorkerInput) -> WorkerOutput {
    WorkerOutput {
        binding: input.binding.clone(),
        outcome: WorkerOutcome::Rejected(ExecutionFailure::Isolation),
    }
}
