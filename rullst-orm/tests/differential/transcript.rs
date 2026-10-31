//! A backend's normalized observations, in the order the scenarios made them.

use std::collections::BTreeMap;
use std::path::Path;

use rullst_orm::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Transcript {
    pub backend: String,
    /// Server version and collation, printed with the report, never compared.
    pub metadata: BTreeMap<String, String>,
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    /// `scenario.step`, unique within a transcript.
    pub key: String,
    pub value: Value,
}

impl Transcript {
    pub fn new(backend: &str) -> Self {
        Self {
            backend: backend.to_string(),
            metadata: BTreeMap::new(),
            steps: Vec::new(),
        }
    }

    pub fn scenario(&mut self, name: &'static str) -> Scenario<'_> {
        Scenario {
            transcript: self,
            name,
        }
    }

    pub fn write(&self, path: &Path) {
        let text = serde_json::to_string_pretty(self).expect("serialize the transcript");
        std::fs::write(path, text).expect("write the transcript");
    }

    pub fn read(path: &Path) -> Self {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
    }
}

/// Records the steps of one scenario.
pub struct Scenario<'a> {
    transcript: &'a mut Transcript,
    name: &'static str,
}

impl Scenario<'_> {
    pub fn record(&mut self, step: &str, value: impl Serialize) {
        let key = format!("{}.{step}", self.name);
        assert!(
            self.transcript
                .steps
                .iter()
                .all(|recorded| recorded.key != key),
            "step {key} is recorded twice"
        );
        let value = serde_json::to_value(value).expect("serialize the observation");
        self.transcript.steps.push(Step { key, value });
    }

    /// Records the value of a successful operation or the kind of its error.
    pub fn outcome<T: Serialize>(&mut self, step: &str, result: Result<T, Error>) {
        match result {
            Ok(value) => self.record(step, value),
            Err(error) => {
                // The message is not compared; it explains a mismatch in the log.
                eprintln!("{} {}.{step}: {error}", self.transcript.backend, self.name);
                self.record(step, error_kind(&error));
            }
        }
    }
}

/// The error variant without its message, plus the constraint name of a
/// violation (only PostgreSQL reports one).
pub fn error_kind(error: &Error) -> Value {
    let kind = match error {
        Error::RecordNotFound => "RecordNotFound",
        Error::DatabaseError(_) => "DatabaseError",
        Error::UniqueViolation { constraint, .. } => {
            return json!({"error": "UniqueViolation", "constraint": constraint});
        }
        Error::ForeignKeyViolation { constraint, .. } => {
            return json!({"error": "ForeignKeyViolation", "constraint": constraint});
        }
        Error::Validation(_) => "Validation",
        Error::SerializationError(_) => "SerializationError",
        Error::Internal(_) => "Internal",
        other => return json!({"error": format!("{other:?}")}),
    };
    json!({ "error": kind })
}
