//! The data of one `audit --report` run, shared by every renderer.

use crate::generators::audit_compliance::EvidenceStatus;
use crate::ui::error_report::sanitize;

use super::catalog::{AsvsRef, Spec};

/// Which part of the report a check belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    Security,
    Accessibility,
}

impl Group {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Security => "security",
            Self::Accessibility => "accessibility",
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Security => "Security checks",
            Self::Accessibility => "Accessibility checks",
        }
    }
}

/// One located observation. Messages, paths and previews are sanitized on
/// construction (secrets masked, control characters replaced, length
/// bounded), so no renderer can emit a raw value or an ANSI sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Finding {
    pub file: String,
    pub line: Option<usize>,
    pub message: String,
    /// A redacted preview (first four characters and `…`), secrets only.
    pub preview: Option<String>,
}

impl Finding {
    pub(crate) fn new(
        file: impl Into<String>,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            file: sanitize(&file.into()),
            line,
            message: sanitize(&message.into()),
            preview: None,
        }
    }

    pub(crate) fn with_preview(mut self, preview: String) -> Self {
        self.preview = Some(sanitize(&preview));
        self
    }

    /// `file:line`, `file` or empty.
    pub(crate) fn location(&self) -> String {
        match self.line {
            Some(line) if !self.file.is_empty() => format!("{}:{line}", self.file),
            _ => self.file.clone(),
        }
    }
}

/// One executed check with its evidence.
#[derive(Clone, Debug)]
pub(crate) struct Check {
    pub spec: &'static Spec,
    pub status: EvidenceStatus,
    pub detail: String,
    pub findings: Vec<Finding>,
}

impl Check {
    /// `FINDINGS` when `findings` is not empty, else `NO FINDINGS`.
    pub(crate) fn from_findings(
        spec: &'static Spec,
        findings: Vec<Finding>,
        detail: impl Into<String>,
    ) -> Self {
        let status = if findings.is_empty() {
            EvidenceStatus::NoFindings
        } else {
            EvidenceStatus::Findings(findings.len())
        };
        Self::with_status(spec, status, findings, detail)
    }

    pub(crate) fn with_status(
        spec: &'static Spec,
        status: EvidenceStatus,
        findings: Vec<Finding>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            spec,
            status,
            detail: sanitize(&detail.into()),
            findings,
        }
    }

    pub(crate) fn not_checked(spec: &'static Spec, reason: &'static str) -> Self {
        Self::with_status(spec, EvidenceStatus::NotChecked(reason), Vec::new(), reason)
    }

    pub(crate) fn asvs(&self) -> &'static [AsvsRef] {
        self.spec.asvs
    }
}

/// One model field in the personal-data inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PersonalField {
    pub model: String,
    pub field: String,
    pub file: String,
    pub line: usize,
    /// `personal`, `encrypted`, `masked` or `review` (name heuristic).
    pub classification: String,
    pub encrypted: bool,
}

/// The personal-data inventory: informational, it never fails the audit.
#[derive(Clone, Debug)]
pub(crate) struct PersonalData {
    pub status: EvidenceStatus,
    pub fields: Vec<PersonalField>,
    pub detail: String,
}

impl PersonalData {
    pub(crate) fn review_count(&self) -> usize {
        self.fields
            .iter()
            .filter(|field| field.classification == "review")
            .count()
    }
}

/// A complete report.
#[derive(Clone, Debug)]
pub(crate) struct Report {
    pub generated_at: String,
    pub checks: Vec<Check>,
    pub personal_data: PersonalData,
}

impl Report {
    /// Non-zero exit: a check reported `FINDINGS` or `ERROR`. The inventory
    /// is informational and does not count.
    pub(crate) fn fails(&self) -> bool {
        self.checks.iter().any(|check| check.status.fails())
    }

    pub(crate) fn failing(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.status.fails())
            .count()
    }
}
