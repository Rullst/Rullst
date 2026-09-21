use crate::{
    CaseOutput, ContentHash, ExecutionFailure, LabError as Error, MAX_CASES, WorkerOutcome,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ResultEvidence {
    /// Explicit local simulation; never execution evidence or academic credit.
    Simulation,
    Experimental {
        receipt: ContentHash,
        observations: ContentHash,
    },
}
/// Minimized feedback by case position; no input, expected value or raw output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseGrade {
    Passed,
    WrongAnswer,
    Trapped(crate::TrapKind),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum JobResult {
    Graded {
        cases: Vec<CaseGrade>,
        passed: u16,
        total: u16,
        artifact: ContentHash,
        evidence: ResultEvidence,
    },
    Rejected {
        failure: ExecutionFailure,
        diagnostics: Option<crate::Diagnostic>,
        evidence: ResultEvidence,
    },
}
impl JobResult {
    pub fn evidence(&self) -> &ResultEvidence {
        match self {
            Self::Graded { evidence, .. } | Self::Rejected { evidence, .. } => evidence,
        }
    }
    pub(super) fn validate(&self) -> Result<(), Error> {
        if let Self::Graded {
            passed,
            total,
            cases,
            ..
        } = self
            && (*total == 0
                || usize::from(*total) > MAX_CASES
                || passed > total
                || cases.len() != usize::from(*total)
                || cases.iter().filter(|c| **c == CaseGrade::Passed).count()
                    != usize::from(*passed))
        {
            return Err(Error::Integrity);
        }
        Ok(())
    }
}
pub(super) fn grade(
    exercise: &crate::Exercise,
    outcome: &WorkerOutcome,
    evidence: ResultEvidence,
) -> Result<JobResult, Error> {
    match outcome {
        WorkerOutcome::CompileRejected { diagnostics } => Ok(JobResult::Rejected {
            failure: ExecutionFailure::Compile,
            diagnostics: Some(diagnostics.clone()),
            evidence,
        }),
        WorkerOutcome::Rejected(failure) => Ok(JobResult::Rejected {
            diagnostics: None,
            failure: *failure,
            evidence,
        }),
        WorkerOutcome::Executed { artifact, cases } => {
            if cases.len() != exercise.grader_cases().len() {
                return Err(Error::Protocol);
            }
            let feedback: Vec<_> = cases
                .iter()
                .zip(exercise.grader_cases())
                .map(|(actual, case)| match actual {
                    CaseOutput::Value(value) if *value == case.expected => CaseGrade::Passed,
                    CaseOutput::Value(_) => CaseGrade::WrongAnswer,
                    CaseOutput::Trap(kind) => CaseGrade::Trapped(*kind),
                })
                .collect();
            let passed = feedback
                .iter()
                .filter(|grade| **grade == CaseGrade::Passed)
                .count();
            Ok(JobResult::Graded {
                cases: feedback,
                passed: u16::try_from(passed).map_err(|_| Error::Capacity)?,
                total: u16::try_from(cases.len()).map_err(|_| Error::Capacity)?,
                artifact: artifact.clone(),
                evidence,
            })
        }
    }
}
