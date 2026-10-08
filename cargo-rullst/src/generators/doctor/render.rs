//! Human-readable doctor output: groups of ✓/!/✗/· lines, a fix and a docs
//! link under each problem, and a one-line summary.

use super::report::{Report, Status};
use crate::ui::style::{self, Style};

fn status_color(status: Status) -> style::Rgb {
    match status {
        Status::Pass => style::PASS,
        Status::Warn => style::WARN,
        Status::Fail => style::FAIL,
        Status::Info => style::MUTED,
    }
}

pub(crate) fn header(style: Style) -> String {
    format!(
        "{} {}\n\n",
        style.bold("Rullst doctor", style::BRIGHT),
        style.paint(&format!("· v{}", env!("CARGO_PKG_VERSION")), style::MUTED)
    )
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

pub(crate) fn render(report: &Report, style: Style) -> String {
    let width = report
        .groups
        .iter()
        .flat_map(|group| &group.checks)
        .map(|check| check.title.chars().count())
        .max()
        .unwrap_or(0)
        + 2;
    let mut out = String::new();
    for group in &report.groups {
        out.push_str(&format!("{}\n", style.bold(group.title, style::BRIGHT)));
        for check in &group.checks {
            let color = status_color(check.status);
            let title = format!("{:<width$}", check.title);
            out.push_str(&format!(
                "  {} {}{}\n",
                style.bold(check.status.glyph(), color),
                style.paint(&title, style::BRIGHT),
                style.paint(&check.detail, style::MUTED)
            ));
            if check.status == Status::Pass {
                continue;
            }
            let label = if check.status == Status::Info {
                "hint"
            } else {
                "fix "
            };
            if let Some(fix) = &check.fix {
                // A multi-line fix (a config snippet) stays aligned under its first line.
                out.push_str(&format!(
                    "      {}  {}\n",
                    style.paint(label, style::MUTED),
                    style.paint(&fix.replace('\n', "\n            "), style::BRIGHT)
                ));
            }
            if let Some(docs) = &check.docs {
                out.push_str(&format!(
                    "      {}  {}\n",
                    style.paint("docs", style::MUTED),
                    style.paint(docs, style::ACCENT)
                ));
            }
        }
        out.push('\n');
    }
    let summary = report.summary;
    let parts = [
        (
            plural(summary.pass, "passed", "passed"),
            style::PASS,
            summary.pass,
        ),
        (
            plural(summary.warn, "warning", "warnings"),
            style::WARN,
            summary.warn,
        ),
        (
            plural(summary.fail, "failed", "failed"),
            style::FAIL,
            summary.fail,
        ),
        (format!("{} info", summary.info), style::MUTED, summary.info),
    ];
    let rendered: Vec<String> = parts
        .iter()
        .map(|(text, color, count)| {
            if *count == 0 {
                style.paint(text, style::MUTED)
            } else {
                style.bold(text, *color)
            }
        })
        .collect();
    out.push_str(&format!(
        "{}  {}\n",
        style.bold("Summary", style::BRIGHT),
        rendered.join(&style.paint(" · ", style::MUTED))
    ));
    if !report.ok {
        out.push_str(&format!(
            "{}\n",
            style.paint(
                "Fix the ✗ items above, then run `cargo rullst doctor` again.",
                style::MUTED
            )
        ));
    } else if summary.warn > 0 {
        out.push_str(&format!(
            "{}\n",
            style.paint(
                "Warnings do not fail the doctor; the fix lines above resolve them.",
                style::MUTED
            )
        ));
    }
    out
}
