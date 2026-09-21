use crate::{LabError, MAX_DIAGNOSTIC_BYTES};
use serde::{Deserialize, Serialize};
/// Bounded untrusted compiler text. Escape it at HTML/terminal rendering; it can
/// quote learner source and must not enter routine logs or metric labels.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Diagnostic(String);
impl Diagnostic {
    pub fn new(text: impl Into<String>) -> Result<Self, LabError> {
        let text = text.into();
        if text.len() > MAX_DIAGNOSTIC_BYTES
            || text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
        {
            return Err(LabError::InvalidInput);
        }
        Ok(Self(text))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Debug for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Diagnostic([redacted])")
    }
}
impl TryFrom<String> for Diagnostic {
    type Error = LabError;
    fn try_from(value: String) -> Result<Self, LabError> {
        Self::new(value)
    }
}
impl From<Diagnostic> for String {
    fn from(value: Diagnostic) -> Self {
        value.0
    }
}
