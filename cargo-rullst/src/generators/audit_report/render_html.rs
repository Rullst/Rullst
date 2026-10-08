//! A single self-contained HTML report: inline CSS (light and dark through
//! `prefers-color-scheme`), no script and no external asset. Every
//! interpolated value goes through [`escape`].

use std::fmt::Write as _;

use super::catalog::{
    ASVS_NAME, ASVS_SOURCE, ASVS_VERSION, MAPPING_NOTE, NOTICE, PERSONAL_DATA, WCAG_VERSION,
    describe, level1_coverage, not_evaluated,
};
use super::model::{Check, Group, Report};

/// Escapes text for element content and quoted attribute values.
pub(super) fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

const STYLE: &str = r#"
:root { color-scheme: light dark; --bg: #f8fafc; --fg: #0f172a; --muted: #475569; --card: #ffffff; --line: #e2e8f0; --ok: #15803d; --warn: #b45309; --bad: #b91c1c; --link: #1d4ed8; }
@media (prefers-color-scheme: dark) { :root { --bg: #020617; --fg: #e2e8f0; --muted: #94a3b8; --card: #0f172a; --line: #1e293b; --ok: #4ade80; --warn: #fbbf24; --bad: #f87171; --link: #93c5fd; } }
* { box-sizing: border-box; }
body { margin: 0; padding: 24px 16px; background: var(--bg); color: var(--fg); font: 15px/1.5 system-ui, -apple-system, "Segoe UI", sans-serif; }
main { max-width: 1100px; margin: 0 auto; }
h1 { font-size: 1.6rem; margin: 0 0 8px; }
h2 { font-size: 1.2rem; margin: 32px 0 8px; }
.notice { border-left: 4px solid var(--warn); background: var(--card); padding: 12px 16px; border-radius: 6px; }
.meta { color: var(--muted); padding-left: 18px; }
.table { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; background: var(--card); border: 1px solid var(--line); font-size: 0.9rem; }
th, td { text-align: left; vertical-align: top; padding: 8px 10px; border-bottom: 1px solid var(--line); overflow-wrap: anywhere; }
th { color: var(--muted); font-weight: 600; }
a { color: var(--link); }
code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 0.85em; }
.status { font-weight: 700; white-space: nowrap; }
.ok { color: var(--ok); } .warn { color: var(--warn); } .bad { color: var(--bad); }
"#;

fn status_class(check: &Check) -> &'static str {
    use crate::generators::audit_compliance::EvidenceStatus as Status;
    match check.status {
        Status::NoFindings => "ok",
        Status::Findings(_) | Status::Error(_) => "bad",
        _ => "warn",
    }
}

fn mapping(check: &Check) -> String {
    let mut parts: Vec<String> = check.spec.asvs.iter().map(describe).collect();
    if let Some(chapter) = check.spec.chapter {
        parts.push(format!("chapter {chapter}"));
    }
    parts.extend(
        check
            .spec
            .wcag
            .iter()
            .map(|criterion| format!("WCAG {criterion}")),
    );
    parts
        .iter()
        .map(|part| escape(part))
        .collect::<Vec<_>>()
        .join("<br>")
}

fn checks_section(out: &mut String, report: &Report, group: Group) {
    let checks: Vec<_> = report
        .checks
        .iter()
        .filter(|check| check.spec.group == group)
        .collect();
    let _ = writeln!(out, "<h2>{}</h2>", escape(group.title()));
    out.push_str("<div class=\"table\"><table><thead><tr><th>Check</th><th>Result</th><th>Evidence</th><th>Mapping</th><th>Fix</th></tr></thead><tbody>\n");
    for check in &checks {
        let _ = writeln!(
            out,
            "<tr><td>{}<br><a href=\"{}\">guide</a></td><td class=\"status {}\">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape(check.spec.title),
            escape(&check.spec.docs()),
            status_class(check),
            escape(check.status.label()),
            escape(&check.detail),
            mapping(check),
            escape(check.spec.fix),
        );
    }
    out.push_str("</tbody></table></div>\n");
    let findings: Vec<_> = checks
        .iter()
        .flat_map(|check| check.findings.iter().map(move |finding| (check, finding)))
        .collect();
    if findings.is_empty() {
        return;
    }
    let _ = writeln!(out, "<h2>{} findings</h2>", escape(group.title()));
    out.push_str("<div class=\"table\"><table><thead><tr><th>Check</th><th>Location</th><th>Finding</th><th>Preview</th></tr></thead><tbody>\n");
    for (check, finding) in findings {
        let _ = writeln!(
            out,
            "<tr><td>{}</td><td><code>{}</code></td><td>{}</td><td><code>{}</code></td></tr>",
            escape(check.spec.title),
            escape(&finding.location()),
            escape(&finding.message),
            escape(finding.preview.as_deref().unwrap_or("")),
        );
    }
    out.push_str("</tbody></table></div>\n");
}

pub(crate) fn render(report: &Report) -> String {
    let (mapped, total) = level1_coverage(report);
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>Rullst Security Report</title>\n<style>");
    out.push_str(STYLE);
    out.push_str("</style>\n</head>\n<body>\n<main>\n<h1>Rullst Security Report</h1>\n");
    let _ = writeln!(out, "<p class=\"notice\">{}</p>", escape(NOTICE));
    out.push_str("<ul class=\"meta\">\n");
    let _ = writeln!(
        out,
        "<li>Generated at: {}</li>",
        escape(&report.generated_at)
    );
    let _ = writeln!(
        out,
        "<li>Generated by: cargo-rullst {} (<code>cargo rullst audit --report html</code>)</li>",
        escape(env!("CARGO_PKG_VERSION"))
    );
    let _ = writeln!(
        out,
        "<li>Standard: {} {}, Level 1 mapping (<a href=\"{}\">source</a>)</li>",
        escape(ASVS_NAME),
        escape(ASVS_VERSION),
        escape(ASVS_SOURCE)
    );
    let _ = writeln!(
        out,
        "<li>Level 1 requirements related to a check: {mapped} of {total}; the rest are listed as NOT EVALUATED</li>"
    );
    let _ = writeln!(
        out,
        "<li>Accessibility criteria: {}</li>",
        escape(WCAG_VERSION)
    );
    let _ = writeln!(
        out,
        "<li>Exit status: {}</li>\n</ul>",
        if report.fails() {
            "non-zero (a check reported FINDINGS or ERROR)"
        } else {
            "zero (no check reported FINDINGS or ERROR)"
        }
    );
    let _ = writeln!(out, "<p>{}</p>", escape(MAPPING_NOTE));
    checks_section(&mut out, report, Group::Security);

    let personal = &report.personal_data;
    out.push_str("<h2>Personal-data inventory</h2>\n");
    let _ = writeln!(
        out,
        "<p><span class=\"status warn\">{}</span> {} <a href=\"{}\">guide</a></p>",
        escape(personal.status.label()),
        escape(&personal.detail),
        escape(&PERSONAL_DATA.docs())
    );
    if !personal.fields.is_empty() {
        out.push_str("<div class=\"table\"><table><thead><tr><th>Model</th><th>Field</th><th>Location</th><th>Classification</th><th>Encrypted</th></tr></thead><tbody>\n");
        for field in &personal.fields {
            let _ = writeln!(
                out,
                "<tr><td>{}</td><td><code>{}</code></td><td><code>{}:{}</code></td><td>{}</td><td>{}</td></tr>",
                escape(&field.model),
                escape(&field.field),
                escape(&field.file),
                field.line,
                escape(&field.classification),
                if field.encrypted { "yes" } else { "no" },
            );
        }
        out.push_str("</tbody></table></div>\n");
    }

    checks_section(&mut out, report, Group::Accessibility);

    out.push_str("<h2>ASVS 5.0.0 Level 1 requirements NOT EVALUATED</h2>\n<p>A static source scan cannot evaluate these requirements; they need design review, dynamic testing or deployment evidence.</p>\n");
    out.push_str("<div class=\"table\"><table><thead><tr><th>Chapter</th><th>Status</th><th>Requirements</th></tr></thead><tbody>\n");
    for chapter in not_evaluated(report) {
        let _ = writeln!(
            out,
            "<tr><td>{} {}</td><td class=\"status warn\">NOT EVALUATED</td><td>{}{}</td></tr>",
            escape(chapter.chapter),
            escape(chapter.name),
            if chapter.whole_chapter {
                "all Level 1: "
            } else {
                ""
            },
            escape(&chapter.requirements.join(", ")),
        );
    }
    out.push_str("</tbody></table></div>\n<p>V16 Security Logging and Error Handling and V17 WebRTC have no Level 1 requirements.</p>\n</main>\n</body>\n</html>\n");
    out
}
