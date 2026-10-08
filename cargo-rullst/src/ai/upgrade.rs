//! `cargo rullst ai upgrade`: the dry-run framework upgrade plan, then a
//! reviewed session grounded in its source findings.
//!
//! The deterministic plan (`cargo rullst upgrade --dry-run`) decides what to
//! fix; the assistant only proposes edits for those findings through the
//! normal action protocol (diff, confirmation, git checkpoint, `cargo
//! check`). Dependency manifests and `Cargo.lock` stay owned by
//! `cargo rullst upgrade`. Trusted instructions carry only the migration rows
//! the findings reference; the findings and the affected files are untrusted
//! data.

use super::prompt::data;
use crate::generators::build::{AssistFinding, AssistPlan};
use std::collections::BTreeSet;
use std::path::Path;

/// Starts the upgrade section of the system prompt (and tells the offline
/// assistant it is in an upgrade session).
pub(super) const UPGRADE_HEADING: &str = "# Framework upgrade session";
/// Labels the untrusted findings block.
pub(super) const FINDINGS_SOURCE: &str = "upgrade-findings";
/// The user goal of the session.
pub(super) const GOAL: &str = "Fix the upgrade findings in order, must-change findings first, with reviewed edits; then run cargo check.";

/// Findings sent to the model in one session.
const MAX_FINDINGS: usize = 40;
const MAX_FILE_BYTES: u64 = 64 * 1024;
const MAX_EXCERPT_BYTES: usize = 16 * 1024;
const MAX_ATTACHED_BYTES: usize = 96 * 1024;

/// Everything the session needs from the plan.
pub(super) struct Brief {
    /// Trusted instructions appended to the system prompt.
    pub instructions: String,
    /// Untrusted data blocks sent with the goal.
    pub attachments: Vec<String>,
    /// Notes shown to the user before the session starts.
    pub notes: Vec<String>,
}

fn ordered(plan: &AssistPlan) -> Vec<&AssistFinding> {
    let mut findings: Vec<&AssistFinding> = plan.findings.iter().collect();
    findings.sort_by_key(|finding| !finding.must_change);
    findings.truncate(MAX_FINDINGS);
    findings
}

/// The text of `line` (1-based) in a project file, trimmed.
fn line_text(root: &Path, finding: &AssistFinding) -> Option<String> {
    let target = super::paths::resolve(root, &finding.path).ok()?;
    let metadata = std::fs::symlink_metadata(&target.absolute).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return None;
    }
    let text = std::fs::read_to_string(&target.absolute).ok()?;
    let line = text.lines().nth(finding.line.checked_sub(1)?)?.trim();
    (!line.is_empty()).then(|| line.chars().take(240).collect())
}

fn instructions(plan: &AssistPlan, findings: &[&AssistFinding]) -> String {
    let mut text = format!(
        "{UPGRADE_HEADING}\n\nThis session upgrades the project to Rullst {}. The deterministic plan \
(rule catalog `{}`) produced the findings in the `{FINDINGS_SOURCE}` data block; the files they \
point to are attached as data when the path policy allows it.\n\n\
- Work through the findings in order, must-change first. For each, propose the smallest \
`edit_file` fix for text you were shown, or say briefly why it needs no change or needs the \
user (configuration, deployment, database or provider settings).\n\
- Follow the migration rows below; do not invent APIs. Keep behaviour unless the row says \
otherwise.\n\
- Never edit Rullst dependency versions: `cargo rullst upgrade` owns Cargo.toml requirements and \
Cargo.lock.\n\
- After your edits, propose `cargo check` (action `cargo` with args `[\"check\"]`).\n",
        plan.target, plan.catalog
    );
    if plan.pending_changes > 0 {
        text.push_str(&format!(
            "- {} Rullst requirement(s) still point at the previous release. Fixes that need the new \
API compile only after `cargo rullst upgrade` (use `--keep-on-failure` to keep a failing \
upgrade for repair); say so when a fix depends on it.\n",
            plan.pending_changes
        ));
    }
    text.push_str("\n## Migration rows for these findings\n\n");
    let mut seen = BTreeSet::new();
    for finding in findings {
        if seen.insert(finding.code) {
            text.push_str(&format!(
                "- {} — {} ({}): {}\n",
                finding.code, finding.row, finding.label, finding.guidance
            ));
        }
    }
    text
}

fn findings_block(root: &Path, plan: &AssistPlan, findings: &[&AssistFinding]) -> String {
    let mut lines = Vec::new();
    for (index, finding) in findings.iter().enumerate() {
        lines.push(format!(
            "{}. {} {} at {}:{}",
            index + 1,
            finding.label,
            finding.code,
            finding.path,
            finding.line
        ));
        lines.push(format!("   message: {}", finding.message));
        if let Some(text) = line_text(root, finding) {
            lines.push(format!("   line: {text}"));
        }
    }
    if plan.findings.len() > findings.len() {
        lines.push(format!(
            "{} more finding(s) are listed by `cargo rullst upgrade --dry-run`.",
            plan.findings.len() - findings.len()
        ));
    }
    data(FINDINGS_SOURCE, &lines.join("\n"), 32 * 1024)
}

/// A whole file when small, otherwise the lines around its findings.
pub(super) fn excerpt(text: &str, lines: &[usize]) -> String {
    if text.len() <= MAX_EXCERPT_BYTES {
        return text.to_string();
    }
    let all: Vec<&str> = text.lines().collect();
    let mut parts = Vec::new();
    for line in lines {
        let start = line.saturating_sub(41);
        let end = (line + 40).min(all.len());
        if start < end {
            parts.push(format!(
                "[lines {}-{}]\n{}",
                start + 1,
                end,
                all[start..end].join("\n")
            ));
        }
    }
    parts.join("\n")
}

/// Builds the session brief from a plan with findings.
pub(super) fn brief(root: &Path, plan: &AssistPlan) -> Brief {
    let findings = ordered(plan);
    let mut attachments = vec![findings_block(root, plan, &findings)];
    let mut notes = Vec::new();
    let mut attached = 0usize;
    let mut paths: Vec<&str> = Vec::new();
    for finding in &findings {
        if !paths.contains(&finding.path.as_str()) {
            paths.push(&finding.path);
        }
    }
    let mut skipped = Vec::new();
    for path in paths {
        let text = super::paths::resolve(root, path)
            .ok()
            .filter(|target| target.exists)
            .and_then(|target| {
                std::fs::symlink_metadata(&target.absolute)
                    .ok()
                    .filter(|metadata| metadata.is_file() && metadata.len() <= 4 * MAX_FILE_BYTES)
                    .and_then(|_| std::fs::read_to_string(&target.absolute).ok())
            });
        let Some(text) = text else {
            skipped.push(path.to_string());
            continue;
        };
        let lines: Vec<usize> = findings
            .iter()
            .filter(|finding| finding.path == path)
            .map(|finding| finding.line)
            .collect();
        let block = data(
            &format!("file {path}"),
            &excerpt(&text, &lines),
            MAX_EXCERPT_BYTES,
        );
        if attached + block.len() > MAX_ATTACHED_BYTES {
            skipped.push(path.to_string());
            continue;
        }
        attached += block.len();
        attachments.push(block);
    }
    if !skipped.is_empty() {
        notes.push(format!(
            "Not shared with the assistant (protected path, not text or over the size limit): {}. Share a file with /add <path> in the chat if needed.",
            skipped.join(", ")
        ));
    }
    Brief {
        instructions: instructions(plan, &findings),
        attachments,
        notes,
    }
}

#[cfg(test)]
#[path = "tests/upgrade.rs"]
mod tests;
