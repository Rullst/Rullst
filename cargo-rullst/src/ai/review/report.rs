//! The review answer: parsed defensively from the model's `rullst-review`
//! block, rendered as text or as the stable `rullst.ai-review.v1` JSON.
//! Action blocks in the answer are counted and ignored, never executed.

use super::super::term::sanitize;
use serde::Serialize;
use serde_json::Value;

/// Version of the JSON report.
pub(crate) const SCHEMA: &str = "rullst.ai-review.v1";
/// Info string of the block the model answers with.
pub(crate) const FENCE: &str = "```rullst-review";
/// Most findings kept from one answer.
const MAX_FINDINGS: usize = 50;
const MAX_TITLE: usize = 200;
const MAX_TEXT: usize = 2_000;
const MAX_FILE: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Severity {
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "critical" | "high" => Self::High,
            "medium" | "moderate" => Self::Medium,
            "low" => Self::Low,
            _ => Self::Info,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
            Self::Info => "INFO",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Finding {
    pub file: String,
    pub line: Option<u64>,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub suggestion: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Parsed {
    pub findings: Vec<Finding>,
    pub summary: Option<String>,
    /// `rullst-action` blocks in the answer, which a review ignores.
    pub ignored_actions: usize,
    /// Why the answer was not in the review format.
    pub format_error: Option<String>,
}

fn bounded(value: Option<&Value>, max: usize) -> String {
    let text = value.and_then(Value::as_str).unwrap_or("").trim();
    let mut output: String = text.chars().take(max).collect();
    if output.len() < text.len() {
        output.push('…');
    }
    sanitize(&output)
}

/// The JSON body of the first `rullst-review` block, else of a `json`
/// block, else the whole answer.
fn review_body(answer: &str) -> &str {
    for fence in [FENCE, "```json"] {
        if let Some(start) = answer.find(fence) {
            let body = &answer[start + fence.len()..];
            let body = body.strip_prefix('\n').unwrap_or(body);
            return body.find("```").map_or(body, |end| &body[..end]);
        }
    }
    answer
}

/// Parses a model answer.
pub(crate) fn parse(answer: &str) -> Parsed {
    let ignored_actions = super::super::protocol::parse(answer).len();
    let mut parsed = Parsed {
        ignored_actions,
        ..Parsed::default()
    };
    let value: Value = match serde_json::from_str(review_body(answer).trim()) {
        Ok(value) => value,
        Err(_) => {
            parsed.format_error = Some("the answer has no rullst-review JSON block".to_string());
            return parsed;
        }
    };
    let Some(findings) = value.get("findings").and_then(Value::as_array) else {
        parsed.format_error = Some("the review JSON has no `findings` list".to_string());
        return parsed;
    };
    parsed.summary = value
        .get("summary")
        .and_then(Value::as_str)
        .map(|_| bounded(value.get("summary"), MAX_TEXT))
        .filter(|summary| !summary.is_empty());
    for finding in findings.iter().take(MAX_FINDINGS) {
        let title = bounded(finding.get("title"), MAX_TITLE);
        if title.is_empty() {
            continue;
        }
        parsed.findings.push(Finding {
            file: bounded(finding.get("file"), MAX_FILE),
            line: finding
                .get("line")
                .and_then(Value::as_u64)
                .filter(|line| *line > 0),
            severity: Severity::parse(
                finding
                    .get("severity")
                    .and_then(Value::as_str)
                    .unwrap_or("info"),
            ),
            title,
            detail: bounded(finding.get("detail"), MAX_TEXT),
            suggestion: bounded(finding.get("suggestion"), MAX_TEXT),
        });
    }
    // Most severe first; the model's order is kept within a severity.
    parsed.findings.sort_by_key(|finding| finding.severity);
    parsed
}

/// What was sent, for both renderings.
#[derive(Debug, Serialize)]
pub(crate) struct Truncated {
    pub path: String,
    pub bytes: usize,
    pub sent_bytes: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct ScopeReport {
    pub diff: &'static str,
    pub base: Option<String>,
    pub include_untracked: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct Files {
    pub reviewed: Vec<String>,
    pub omitted: Vec<super::diff::Omitted>,
    pub truncated: Vec<Truncated>,
    /// The git output was cut at its size bound before every file was read.
    pub output_cut: bool,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct UsageReport {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    /// The usage line shown in the text report (with any cost estimate).
    pub line: Option<String>,
}

/// The complete review, serialized as `rullst.ai-review.v1`.
#[derive(Debug, Serialize)]
pub(crate) struct Report {
    pub schema: &'static str,
    pub provider: String,
    pub offline: bool,
    pub scope: ScopeReport,
    pub files: Files,
    pub redactions: usize,
    pub findings: Vec<Finding>,
    pub summary: Option<String>,
    pub ignored_actions: usize,
    pub format_error: Option<String>,
    /// The answer as text, only when it was not in the review format.
    pub raw_answer: Option<String>,
    pub usage: UsageReport,
}

fn location(finding: &Finding) -> String {
    match (finding.file.is_empty(), finding.line) {
        (true, _) => "(no file)".to_string(),
        (false, Some(line)) => format!("{}:{line}", finding.file),
        (false, None) => finding.file.clone(),
    }
}

/// The text rendering, without the header lines printed before the request.
pub(crate) fn render_text(report: &Report) -> String {
    let mut lines = Vec::new();
    if report.findings.is_empty() && report.format_error.is_none() {
        lines.push("No findings.".to_string());
    }
    for (index, finding) in report.findings.iter().enumerate() {
        lines.push(format!(
            "{}. [{}] {} · {}",
            index + 1,
            finding.severity.label(),
            location(finding),
            finding.title
        ));
        if !finding.detail.is_empty() {
            lines.push(format!("   {}", finding.detail));
        }
        if !finding.suggestion.is_empty() {
            lines.push(format!("   Suggested fix: {}", finding.suggestion));
        }
    }
    if let Some(error) = &report.format_error {
        lines.push(format!(
            "The model did not answer in the review format ({error}); its answer follows as text:"
        ));
        lines.push(report.raw_answer.clone().unwrap_or_default());
    }
    if let Some(summary) = &report.summary {
        lines.push(format!("Summary: {summary}"));
    }
    if report.ignored_actions > 0 {
        lines.push(format!(
            "Ignored {} proposed action(s): a review is read-only and never edits or runs anything.",
            report.ignored_actions
        ));
    }
    lines.join("\n")
}
