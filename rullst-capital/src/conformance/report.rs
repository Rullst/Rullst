//! The structured result of a conformance run, and the panic-catching runner
//! of its cases.

use std::fmt;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;
use std::task::{Context, Poll};

/// How one conformance case ended. Since 13.0.0-alpha.5.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseOutcome {
    /// Every expectation of the case held.
    Passed,
    /// An expectation failed, or the provider panicked.
    Failed,
    /// The case needs a fixture the harness does not supply.
    Skipped,
}

/// One case of a conformance run. Since 13.0.0-alpha.5.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConformanceCase {
    id: &'static str,
    outcome: CaseOutcome,
    detail: String,
}

impl ConformanceCase {
    /// The stable case identifier, such as `notification.forged_refused`.
    pub fn id(&self) -> &'static str {
        self.id
    }

    /// How the case ended.
    pub fn outcome(&self) -> CaseOutcome {
        self.outcome
    }

    /// The failed expectations, the reason for a skip, or notes about what a
    /// passed case could not exercise; empty otherwise.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// The result of running a conformance kit against one provider: one
/// [`ConformanceCase`] per case, in a fixed order. Since 13.0.0-alpha.5.
///
/// Its `Display` form lists every case and is the message of
/// [`assert_passed`](Self::assert_passed).
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConformanceReport {
    provider: String,
    cases: Vec<ConformanceCase>,
}

impl ConformanceReport {
    pub(super) fn new(provider: impl Into<String>, cases: Vec<ConformanceCase>) -> Self {
        Self {
            provider: provider.into(),
            cases,
        }
    }

    /// The name of the provider under test.
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Every case, in the order the kit ran them.
    pub fn cases(&self) -> &[ConformanceCase] {
        &self.cases
    }

    /// The case with this identifier.
    pub fn case(&self, id: &str) -> Option<&ConformanceCase> {
        self.cases.iter().find(|case| case.id == id)
    }

    /// Whether no case failed. Skipped cases do not fail a run.
    pub fn passed(&self) -> bool {
        self.failures().next().is_none()
    }

    /// The failed cases.
    pub fn failures(&self) -> impl Iterator<Item = &ConformanceCase> {
        self.cases
            .iter()
            .filter(|case| case.outcome == CaseOutcome::Failed)
    }

    /// Fails the calling test with the whole report when a case failed.
    ///
    /// Meant for `#[test]` functions; production code reads
    /// [`passed`](Self::passed) instead.
    #[track_caller]
    pub fn assert_passed(&self) {
        assert!(self.passed(), "{self}");
    }
}

impl fmt::Display for ConformanceReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = |outcome| {
            self.cases
                .iter()
                .filter(|case| case.outcome == outcome)
                .count()
        };
        writeln!(
            formatter,
            "conformance of {}: {} passed, {} failed, {} skipped",
            self.provider,
            count(CaseOutcome::Passed),
            count(CaseOutcome::Failed),
            count(CaseOutcome::Skipped),
        )?;
        for case in &self.cases {
            let label = match case.outcome {
                CaseOutcome::Passed => "ok",
                CaseOutcome::Failed => "FAILED",
                CaseOutcome::Skipped => "skipped",
            };
            write!(formatter, "  {label} {}", case.id)?;
            if !case.detail.is_empty() {
                write!(formatter, ": {}", case.detail)?;
            }
            writeln!(formatter)?;
        }
        Ok(())
    }
}

/// The expectations one case collected.
#[derive(Default)]
pub(super) struct Check {
    failures: Vec<String>,
    notes: Vec<String>,
    skipped: Option<String>,
}

impl Check {
    /// Records `expectation` as failed unless it `holds`.
    pub(super) fn expect(&mut self, holds: bool, expectation: impl Into<String>) {
        if !holds {
            self.failures.push(expectation.into());
        }
    }

    /// Records what a passing case could not exercise.
    pub(super) fn note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    /// Marks the whole case as not applicable.
    pub(super) fn skip(&mut self, reason: impl Into<String>) {
        self.skipped = Some(reason.into());
    }

    fn finish(self, id: &'static str) -> ConformanceCase {
        let (outcome, detail) = if !self.failures.is_empty() {
            (CaseOutcome::Failed, self.failures.join("; "))
        } else if let Some(reason) = self.skipped {
            (CaseOutcome::Skipped, reason)
        } else {
            (CaseOutcome::Passed, self.notes.join("; "))
        };
        ConformanceCase {
            id,
            outcome,
            detail,
        }
    }
}

/// Runs one case; a panic inside it fails the case instead of the run.
pub(super) async fn run_case(
    id: &'static str,
    check: impl Future<Output = Check>,
) -> ConformanceCase {
    match CatchUnwind(Box::pin(check)).await {
        Ok(check) => check.finish(id),
        Err(message) => ConformanceCase {
            id,
            outcome: CaseOutcome::Failed,
            detail: format!("panicked: {message}"),
        },
    }
}

/// Polls a future inside `catch_unwind`, so a panicking provider becomes a
/// failed case.
struct CatchUnwind<F>(Pin<Box<F>>);

impl<F: Future> Future for CatchUnwind<F> {
    type Output = Result<F::Output, String>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = &mut self.0;
        match catch_unwind(AssertUnwindSafe(|| inner.as_mut().poll(context))) {
            Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(payload) => Poll::Ready(Err(payload
                .downcast_ref::<&str>()
                .map(|message| (*message).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "a non-text panic payload".to_owned()))),
        }
    }
}
