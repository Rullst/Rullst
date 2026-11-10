//! Human, Markdown and `rullst.upgrade-plan.v1` renderings of a plan.

use super::manifest::ManifestUpgradePlan;
use super::relative_report_path;
use super::rules::{self, FindingKind, RuleSelection, SourceScan};
use crate::term::display_path;
use colored::Colorize;
use semver::Version;
use std::path::Path;

/// The published v13 migration guide.
pub(super) const GUIDE: &str = "https://rullst.github.io/Rullst/book/migration-v13.html";
/// Its checklist for 13.0.0-alpha.2 applications.
pub(super) const ALPHA2_CHECKLIST: &str = "upgrading-an-alpha2-application-to-alpha3";
/// Its checklist for 13.0.0-alpha.3 applications, which also explains the
/// rules a later 13.0 prerelease adds for alpha.4 changes.
pub(super) const ALPHA3_CHECKLIST: &str = "upgrading-an-alpha3-application-to-alpha4";

/// The checklist for an application on the 13.0 prerelease `current`, if
/// the guide has one: alpha.2 and alpha.3 have their own; a later alpha,
/// beta or release candidate gets only rules for alpha.4 changes, which the
/// alpha.3 checklist describes. alpha.1 has none.
fn checklist(current: &Version) -> Option<&'static str> {
    if (current.major, current.minor, current.patch) != (13, 0, 0) || current.pre.is_empty() {
        return None;
    }
    let pre = current.pre.as_str();
    match pre.strip_prefix("alpha.").map(str::parse::<u64>) {
        Some(Ok(2)) => Some(ALPHA2_CHECKLIST),
        Some(Ok(number)) if number >= 3 => Some(ALPHA3_CHECKLIST),
        Some(_) => None,
        None if pre.starts_with("alpha") => None,
        None => Some(ALPHA3_CHECKLIST),
    }
}

/// The part of the guide the plan links: the checklist for the project's
/// version, or, for a v12 project, alpha.1, an unknown version or
/// `--all-rules`, the table of changes from 12.1.0 that holds every
/// finding's row.
pub(super) fn guide_url(selection: &RuleSelection) -> String {
    let checklist = selection
        .current
        .as_ref()
        .filter(|_| !selection.all_rules)
        .and_then(checklist);
    match checklist {
        Some(anchor) => format!("{GUIDE}#{anchor}"),
        None => rules::MIGRATION_GUIDE_URL.to_string(),
    }
}

/// What the plan says about the rules it skipped, when it skipped any.
fn selection_note(selection: &RuleSelection) -> Option<String> {
    let current = selection.current.as_ref()?;
    (selection.skipped_rules > 0).then(|| {
        format!(
            "introduced after {current}, the Rullst version this project uses; earlier rules skipped: {} (findings hidden: {}); --all-rules applies every rule",
            selection.skipped_rules, selection.hidden_findings
        )
    })
}

/// The plan as plain text (no colour).
pub(super) fn plan_text(
    root: &Path,
    target: &Version,
    plans: &[ManifestUpgradePlan],
    scan: &SourceScan,
) -> String {
    let mut lines = vec![format!("Project: {}", display_path(root))];
    for plan in plans {
        let relative = relative_report_path(root, &plan.path);
        for change in &plan.changes {
            lines.push(format!(
                "  {}: {} ({}) {} -> {}",
                relative, change.key, change.package, change.from, change.to
            ));
        }
        for warning in &plan.warnings {
            lines.push(format!("  REVIEW {relative}: {warning}"));
        }
    }
    if target.major != 13 {
        return lines.join("\n");
    }
    lines.push(format!(
        "Source findings ({}): {} must-change, {} review",
        rules::RULE_CATALOG_VERSION,
        scan.count(FindingKind::MustChange),
        scan.count(FindingKind::Review)
    ));
    if let Some(note) = selection_note(&scan.selection) {
        lines.push(format!("  Rules: {note}"));
    }
    for finding in &scan.findings {
        lines.push(format!(
            "  {} {}:{} [{}] {}",
            finding.rule.kind.label(),
            relative_report_path(root, &finding.path),
            finding.line,
            finding.rule.code,
            finding.rule.message
        ));
        lines.push(format!("      migration-v13 row: {}", finding.rule.row));
    }
    for path in &scan.unscanned {
        lines.push(format!(
            "  NOT SCANNED {}: not valid Rust, not UTF-8 or larger than 2 MiB; review it by hand",
            relative_report_path(root, path)
        ));
    }
    let must_change = scan.count(FindingKind::MustChange);
    if must_change > 0 {
        lines.push(format!(
            "{must_change} must-change finding(s) stop `cargo rullst upgrade` before it changes anything: fix them first, or pass --keep-on-failure to change the versions anyway and fix them in the kept files."
        ));
    }
    if !scan.findings.is_empty() {
        lines.push(format!("Migration guide: {}", guide_url(&scan.selection)));
    }
    lines.join("\n")
}

pub(super) fn print_plan(
    root: &Path,
    target: &Version,
    plans: &[ManifestUpgradePlan],
    scan: &SourceScan,
    dry_run: bool,
) {
    let mode = if dry_run { "DRY RUN" } else { "APPLY" };
    println!(
        "{}",
        format!("\nRullst assisted upgrade — {mode} — target {target}")
            .cyan()
            .bold()
    );
    println!("{}", plan_text(root, target, plans, scan));
}

pub(super) fn render_report(
    root: &Path,
    target: &Version,
    plans: &[ManifestUpgradePlan],
    scan: &SourceScan,
) -> String {
    let mut report = format!(
        "# Rullst assisted upgrade report\n\n- Project: `{}`\n- Target: `{target}`\n- Scope: dependency manifests, Cargo.lock and compiler-provided Rust fixes\n\n",
        display_path(root)
    );
    report.push_str("## Dependency plan\n\n");
    for plan in plans {
        let relative = relative_report_path(root, &plan.path);
        for change in &plan.changes {
            report.push_str(&format!(
                "- `{}`: `{}` (`{}`) `{}` → `{}`\n",
                relative, change.key, change.package, change.from, change.to
            ));
        }
        for warning in &plan.warnings {
            report.push_str(&format!("- REVIEW `{relative}`: {warning}\n"));
        }
    }
    report.push_str(&format!(
        "\n## Source review\n\nRule catalog `{}`; findings link to rows of the [v13 migration guide]({}).\n\n",
        rules::RULE_CATALOG_VERSION,
        rules::MIGRATION_GUIDE_URL
    ));
    if let Some(note) = selection_note(&scan.selection) {
        report.push_str(&format!("Rules: {note}.\n\n"));
    }
    let guide = guide_url(&scan.selection);
    if guide != rules::MIGRATION_GUIDE_URL {
        report.push_str(&format!(
            "Checklist for this project's Rullst version: {guide}\n\n"
        ));
    }
    if scan.findings.is_empty() {
        report.push_str(
            "No applicable source rules matched. This is not proof of runtime compatibility.\n",
        );
    }
    for finding in &scan.findings {
        report.push_str(&format!(
            "- **{}** `{}` line {} (`{}`): {} — migration row “{}”\n",
            finding.rule.kind.label(),
            relative_report_path(root, &finding.path),
            finding.line,
            finding.rule.code,
            finding.rule.message,
            finding.rule.row
        ));
    }
    for path in &scan.unscanned {
        report.push_str(&format!(
            "- **NOT SCANNED** `{}`: review it by hand\n",
            relative_report_path(root, path)
        ));
    }
    report.push_str(
        "\n## Mandatory manual gates\n\n- Review every diff and the migration guide for the target major.\n- Restore a database backup into a disposable environment and rehearse migrations and rollback.\n- Run formatting, Clippy, the complete application tests, authorization negatives and a production-profile smoke test.\n- Revalidate Nexus, Studio, providers, proxy trust, CSRF/CORS and secrets.\n",
    );
    report
}

/// The `rullst.upgrade-plan.v1` document. v13 extends it additively: each
/// finding keeps `path`, `line`, `code`, `severity` and `message` and adds
/// `kind`, `introduced_in`, `migration_row` and `migration_url`; the root adds
/// `rule_selection`.
pub(super) fn render_json_report(
    root: &Path,
    target: &Version,
    plans: &[ManifestUpgradePlan],
    scan: &SourceScan,
) -> Result<String, serde_json::Error> {
    let manifests = plans
        .iter()
        .map(|plan| {
            serde_json::json!({
                "path": relative_report_path(root, &plan.path),
                "matched_dependencies": plan.matched,
                "source_majors": plan.source_majors,
                "changes": plan.changes,
                "warnings": plan.warnings,
            })
        })
        .collect::<Vec<_>>();
    let findings = scan
        .findings
        .iter()
        .map(|finding| {
            serde_json::json!({
                "path": relative_report_path(root, &finding.path),
                "line": finding.line,
                "code": finding.rule.code,
                "severity": finding.rule.kind.severity(),
                "kind": finding.rule.kind.as_str(),
                "introduced_in": finding.rule.introduced,
                "message": finding.rule.message,
                "migration_row": finding.rule.row,
                "migration_url": rules::MIGRATION_GUIDE_URL,
            })
        })
        .collect::<Vec<_>>();
    let unscanned = scan
        .unscanned
        .iter()
        .map(|path| relative_report_path(root, path))
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&serde_json::json!({
        "schema": "rullst.upgrade-plan.v1",
        "schema_version": "rullst.upgrade-plan.v1",
        "rule_catalog": rules::RULE_CATALOG_VERSION,
        "target": target.to_string(),
        "manifests": manifests,
        "source_findings": findings,
        "finding_counts": {
            "must_change": scan.count(FindingKind::MustChange),
            "review": scan.count(FindingKind::Review),
        },
        "rule_selection": {
            "current_version": scan.selection.current.as_ref().map(Version::to_string),
            "all_rules": scan.selection.all_rules,
            "skipped_rules": scan.selection.skipped_rules,
            "hidden_findings": scan.selection.hidden_findings,
        },
        "unscanned_sources": unscanned,
        "migration_guide": guide_url(&scan.selection),
        "automatic_scope": [
            "workspace dependency manifests",
            "Cargo.lock resolution",
            "compiler-provided Rust fixes",
            "locked cargo check for the selected features"
        ],
        "manual_gates": [
            "review the complete diff",
            "rehearse database restore, migration and rollback",
            "run the full application tests and authorization negatives",
            "validate providers, proxy trust, Nexus, Studio, CSRF/CORS and secrets"
        ],
        "production_ready": false
    }))
}
