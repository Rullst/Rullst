//! Compares a transcript with the SQLite reference, step by step.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::expected::{self, Kind};
use crate::transcript::Transcript;

/// The outcome of one comparison: a readable report and its problems.
pub struct Report {
    lines: Vec<String>,
    pub problems: Vec<String>,
}

impl Report {
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    fn problem(&mut self, problem: String) {
        self.lines.push(format!("  FAIL {problem}"));
        self.problems.push(problem);
    }
}

fn compact(value: &Value) -> String {
    serde_json::to_string(value).expect("serialize a JSON value")
}

/// Two runs of the same backend must record exactly the same transcript.
pub fn identical(first: &Transcript, second: &Transcript) -> Vec<String> {
    let mut problems = Vec::new();
    if first.steps.len() != second.steps.len() {
        problems.push(format!(
            "{} recorded {} steps in one run and {} in the next",
            first.backend,
            first.steps.len(),
            second.steps.len()
        ));
    }
    for (one, two) in first.steps.iter().zip(&second.steps) {
        if one != two {
            problems.push(format!(
                "{} is not deterministic at `{}`: {} then `{}` {}",
                first.backend,
                one.key,
                compact(&one.value),
                two.key,
                compact(&two.value)
            ));
        }
    }
    problems
}

/// Compares `candidate` with `reference` (SQLite). Equal steps pass; a
/// different step passes only when `expected.rs` declares exactly the value
/// the candidate produced. A declared difference that no longer occurs, or
/// that names an unknown step, is a problem too, so the list stays true.
pub fn against_reference(reference: &Transcript, candidate: &Transcript) -> Report {
    let backend = candidate.backend.as_str();
    let mut report = Report {
        lines: vec![format!(
            "{backend} against {}: {}",
            reference.backend,
            describe(candidate)
        )],
        problems: Vec::new(),
    };
    let observed: BTreeMap<&str, &Value> = candidate
        .steps
        .iter()
        .map(|step| (step.key.as_str(), &step.value))
        .collect();
    let (mut equal, mut documented) = (0, 0);
    for step in &reference.steps {
        let declared = expected::lookup(backend, &step.key);
        let Some(&value) = observed.get(step.key.as_str()) else {
            report.problem(format!("{backend} did not record `{}`", step.key));
            continue;
        };
        match declared {
            None if *value == step.value => equal += 1,
            Some(difference) if *value == step.value => report.problem(format!(
                "{backend} now matches {} at `{}`; remove the documented difference ({})",
                reference.backend, step.key, difference.reason
            )),
            Some(difference) if *value == difference.value() => {
                documented += 1;
                let label = match difference.kind {
                    Kind::Provider => "documented",
                    Kind::Defect => "DEFECT",
                };
                report.lines.push(format!(
                    "  {label} `{}`: {} {} / {backend} {} -- {}",
                    step.key,
                    reference.backend,
                    compact(&step.value),
                    compact(value),
                    difference.reason
                ));
            }
            declared => {
                let mut problem = format!(
                    "{backend} differs at `{}`\n      {:<9}{}\n      {backend:<9}{}",
                    step.key,
                    reference.backend,
                    compact(&step.value),
                    compact(value)
                );
                if let Some(difference) = declared {
                    problem.push_str(&format!(
                        "\n      {:<9}{} ({})",
                        "declared",
                        compact(&difference.value()),
                        difference.reason
                    ));
                }
                report.problem(problem);
            }
        }
    }
    let known: BTreeSet<&str> = reference
        .steps
        .iter()
        .map(|step| step.key.as_str())
        .collect();
    for step in &candidate.steps {
        if !known.contains(step.key.as_str()) {
            report.problem(format!(
                "{backend} recorded `{}`, which the {} transcript lacks (is it stale? run the strict-sqlite build again)",
                step.key, reference.backend
            ));
        }
    }
    for key in expected::keys_for(backend) {
        if !known.contains(key) {
            report.problem(format!(
                "a documented difference for {backend} names `{key}`, which no scenario records"
            ));
        }
    }
    report.lines.push(format!(
        "  {equal} equal, {documented} documented difference(s), {} problem(s)",
        report.problems.len()
    ));
    report
}

fn describe(transcript: &Transcript) -> String {
    transcript
        .metadata
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}
