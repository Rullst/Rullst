//! The short grouped terminal summary (✓ / ! / ✗). Colour only through
//! [`Style`], which is plain under `NO_COLOR`, `CI`, `TERM=dumb` or a
//! non-terminal stream.

use std::fmt::Write as _;

use crate::generators::audit_compliance::EvidenceStatus;
use crate::ui::style::{self, Style};

use super::catalog::{ASVS_VERSION, level1_coverage};
use super::model::{Check, Group, Report};

fn mark(status: &EvidenceStatus, style: Style) -> String {
    match status {
        EvidenceStatus::NoFindings => style.paint("✓", style::PASS),
        EvidenceStatus::Findings(_) | EvidenceStatus::Error(_) => style.paint("✗", style::FAIL),
        _ => style.paint("!", style::WARN),
    }
}

fn outcome(check: &Check) -> String {
    match &check.status {
        EvidenceStatus::Findings(count) => format!("{count} finding(s)"),
        EvidenceStatus::NotChecked(reason) => format!("not checked: {reason}"),
        EvidenceStatus::Error(_) => "error: the check could not complete".to_string(),
        other => other.label().to_ascii_lowercase(),
    }
}

fn group(out: &mut String, report: &Report, group: Group, style: Style) {
    let _ = writeln!(out, "{}", style.bold(group.title(), style::BRIGHT));
    for check in report
        .checks
        .iter()
        .filter(|check| check.spec.group == group)
    {
        let _ = writeln!(
            out,
            "  {} {} — {}",
            mark(&check.status, style),
            check.spec.title,
            outcome(check)
        );
        if check.status.fails() {
            let _ = writeln!(out, "      fix: {}", check.spec.fix);
            let _ = writeln!(
                out,
                "      docs: {}",
                style.paint(&check.spec.docs(), style::ACCENT)
            );
        }
    }
}

/// The summary printed after the report file is written.
pub(crate) fn render(report: &Report, written: &str, style: Style) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{}",
        style.bold(
            &format!("Rullst security report (OWASP ASVS {ASVS_VERSION} Level 1 mapping)"),
            style::ACCENT
        )
    );
    let _ = writeln!(
        out,
        "{}",
        style.paint(
            "Evidence for a reviewer; it does not replace a manual review or a penetration test.",
            style::MUTED
        )
    );
    group(&mut out, report, Group::Security, style);
    let personal = &report.personal_data;
    let _ = writeln!(out, "{}", style.bold("Personal data", style::BRIGHT));
    let review = personal.review_count();
    let personal_mark = match (&personal.status, review) {
        (EvidenceStatus::Observed(_), 0) => style.paint("✓", style::PASS),
        _ => style.paint("!", style::WARN),
    };
    let _ = writeln!(
        out,
        "  {personal_mark} Inventory — {} field(s) listed, {review} need review (name heuristic)",
        personal.fields.len()
    );
    group(&mut out, report, Group::Accessibility, style);
    let (mapped, total) = level1_coverage(report);
    let _ = writeln!(
        out,
        "NOT EVALUATED: {} of {total} ASVS Level 1 requirements are outside these static checks.",
        total - mapped
    );
    let _ = writeln!(out, "Report written to {written}");
    let failing = report.failing();
    if failing > 0 {
        let _ = writeln!(
            out,
            "{}",
            style.paint(
                &format!("{failing} check(s) reported FINDINGS or ERROR; exit status 1."),
                style::FAIL
            )
        );
    }
    out
}
