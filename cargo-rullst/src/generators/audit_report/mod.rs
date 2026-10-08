//! `cargo rullst audit --report [md|html|json]`: a static evidence report
//! mapped to OWASP ASVS 5.0.0 Level 1, with a personal-data inventory and
//! accessibility heuristics.
//!
//! The dependency and IDOR checks reuse the regular audit (`cargo audit`
//! with its governed exceptions, and the route-classification scanner). The
//! report never states a pass or a certification: each check reports an
//! [`EvidenceStatus`], and Level 1 requirements outside the checks are listed
//! as `NOT EVALUATED`.

mod a11y;
mod catalog;
mod model;
mod personal_data;
mod render_html;
mod render_json;
mod render_md;
mod secrets;
mod sources;
mod summary;
mod web;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::generators::audit_compliance::EvidenceStatus;
use crate::generators::audit_idor::collect_rust_source_files;
use crate::generators::audit_scope::{
    cargo_audit_status, package_source_roots, scan_each, validate_audit_ignores,
};
use crate::generators::output_guard::write_output;
use crate::ui::error_report::AlreadyReported;
use crate::ui::style::Style;

use catalog::{DEPENDENCIES, IDOR};
use model::{Check, Finding, Report};
use sources::ProjectSources;

/// The report file format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReportFormat {
    Markdown,
    Html,
    Json,
}

impl ReportFormat {
    /// `md`, `html` or `json`, as accepted by `--report`.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "md" => Some(Self::Markdown),
            "html" => Some(Self::Html),
            "json" => Some(Self::Json),
            _ => None,
        }
    }

    pub(crate) fn default_file(self) -> &'static str {
        match self {
            Self::Markdown => "SECURITY_REPORT.md",
            Self::Html => "SECURITY_REPORT.html",
            Self::Json => "SECURITY_REPORT.json",
        }
    }
}

/// The `audit --report` flags.
#[derive(Clone, Debug)]
pub(crate) struct ReportOptions<'a> {
    pub format: ReportFormat,
    pub output: Option<PathBuf>,
    pub ignores: &'a [String],
}

/// The external programs a report runs; tests substitute fixtures.
pub(crate) struct Tools<'a> {
    pub cargo: &'a OsStr,
    pub git: &'a OsStr,
}

impl Tools<'static> {
    pub(crate) fn system() -> Self {
        Self {
            cargo: OsStr::new("cargo"),
            git: OsStr::new("git"),
        }
    }
}

fn dependency_check(root: &Path, cargo: &OsStr, ignores: &[String]) -> Check {
    let status = cargo_audit_status(cargo, root, ignores);
    let detail = match &status {
        EvidenceStatus::NoFindingsOutsideExceptions(ids) => format!(
            "cargo-audit reported no advisory outside the governed exceptions {}; those advisories remain unresolved and need separate review.",
            ids.join(", ")
        ),
        EvidenceStatus::Error(error) => format!(
            "cargo-audit did not complete successfully (advisories found or the run failed); run `cargo audit` for its full output. Last error: {}",
            last_error_line(error)
        ),
        other => other.detail(),
    };
    Check::with_status(&DEPENDENCIES, status, Vec::new(), detail)
}

/// The last `error:` line of a tool's standard error (or its last line), so
/// progress output and workstation paths stay out of the report.
fn last_error_line(stderr: &str) -> &str {
    let mut lines = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    lines
        .clone()
        .rfind(|line| line.starts_with("error"))
        .or_else(|| lines.next_back())
        .unwrap_or("no output")
}

/// `File 'path:line': message` from the IDOR scanner as a located finding.
fn idor_finding(warning: &str) -> Finding {
    let located = warning
        .strip_prefix("File '")
        .and_then(|rest| rest.split_once("': "))
        .and_then(|(location, message)| {
            let (file, line) = location.rsplit_once(':')?;
            Some((file, line.parse().ok()?, message))
        });
    match located {
        Some((file, line, message)) => Finding::new(file, Some(line), message),
        None => Finding::new("", None, warning),
    }
}

fn idor_check(root: &Path) -> Check {
    let roots = package_source_roots(root).unwrap_or_else(|| vec![root.to_path_buf()]);
    let available = roots
        .iter()
        .any(|source| !collect_rust_source_files(source).files.is_empty());
    if !available {
        return Check::not_checked(&IDOR, "no scannable Rust source was available");
    }
    let (_, warnings) = scan_each(&roots, crate::generators::audit::scan_idor_vulnerabilities);
    let findings = warnings
        .iter()
        .map(|warning| idor_finding(warning))
        .collect();
    Check::from_findings(
        &IDOR,
        findings,
        "Parameterized routes need an adjacent `rullst-access` classification and the matching guard somewhere in the crate; the heuristic does not prove the route is mounted behind it.",
    )
}

/// Runs every check against the project in `root`.
pub(crate) fn collect(root: &Path, ignores: &[String], tools: &Tools<'_>) -> Report {
    let sources = ProjectSources::load(root);
    let config = sources::SecurityConfig::load(root);
    let mut checks = vec![
        web::headers(&sources, &config),
        web::csrf(&sources, &config),
        web::cookies(&sources, &config),
        web::rate_limit(&sources),
        secrets::check(root, secrets::tracked_files(tools.git, root)),
        dependency_check(root, tools.cargo, ignores),
        idor_check(root),
    ];
    if !sources.incomplete.is_empty() {
        let reason = format!(
            "the source walk is incomplete ({}); files beyond it were not scanned",
            sources.incomplete.join("; ")
        );
        for check in checks.iter_mut().take(4) {
            check.status = EvidenceStatus::Error(reason.clone());
        }
    }
    let personal_data = personal_data::inventory(&sources);
    let (markup, incomplete) = a11y::collect(root, &sources);
    checks.extend(a11y::checks(&markup, &incomplete));
    Report {
        generated_at: chrono::Utc::now().to_rfc3339(),
        checks,
        personal_data,
    }
}

/// The report text in `format`.
pub(crate) fn render(report: &Report, format: ReportFormat) -> Result<String, serde_json::Error> {
    Ok(match format {
        ReportFormat::Markdown => render_md::render(report),
        ReportFormat::Html => render_html::render(report),
        ReportFormat::Json => render_json::render(report)?,
    })
}

/// Writes the report, prints the grouped summary and fails (exit status 1)
/// when a check reported `FINDINGS` or `ERROR`.
pub(crate) fn run_report(options: ReportOptions<'_>) -> Result<(), Box<dyn std::error::Error>> {
    validate_audit_ignores(options.ignores)?;
    let root = Path::new(".");
    let report = collect(root, options.ignores, &Tools::system());
    let output = options
        .output
        .unwrap_or_else(|| PathBuf::from(options.format.default_file()));
    // Replace a previous report, but never through a symlink in the checkout.
    write_output(&output, render(&report, options.format)?.as_bytes(), true)?;
    print!(
        "{}",
        summary::render(&report, &output.display().to_string(), Style::stdout())
    );
    if report.fails() {
        return Err(AlreadyReported::new(
            1,
            format!(
                "security report: {} check(s) reported FINDINGS or ERROR",
                report.failing()
            ),
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
