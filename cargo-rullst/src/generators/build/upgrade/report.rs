//! Human, Markdown and `rullst.upgrade-plan.v1` renderings of a plan.

use super::manifest::ManifestUpgradePlan;
use super::relative_report_path;
use super::rules::{self, FindingKind, RuleSelection, SourceFinding, SourceScan};
use crate::term::display_path;
use colored::Colorize;
use semver::Version;
use std::collections::BTreeMap;
use std::path::Path;

/// The published v13 migration guide.
pub(super) const GUIDE: &str = "https://rullst.github.io/Rullst/book/migration-v13.html";
/// Its checklist for 13.0.0-alpha.2 applications.
pub(super) const ALPHA2_CHECKLIST: &str = "upgrading-an-alpha2-application-to-alpha3";
/// Its checklist for 13.0.0-alpha.3 applications.
pub(super) const ALPHA3_CHECKLIST: &str = "upgrading-an-alpha3-application-to-alpha4";
/// Its checklist for 13.0.0-alpha.4 applications.
pub(super) const ALPHA4_CHECKLIST: &str = "upgrading-an-alpha4-application-to-alpha5";
/// Its checklist for 13.0.0-alpha.5 applications, which also explains the
/// alpha.6 changes that later rules locate.
pub(super) const ALPHA5_CHECKLIST: &str = "upgrading-an-alpha5-application-to-alpha6";
/// Its checklist for 13.0.0-alpha.6 applications, the changes of
/// 13.0.0-rc.1.
pub(super) const ALPHA6_CHECKLIST: &str = "upgrading-an-alpha6-application-to-rc1";

/// The checklist for an application on the 13.0 prerelease `current`, if
/// the guide has one: alpha.2 to alpha.6 have their own; a later alpha, beta
/// or release candidate gets the alpha.6 one, the latest. alpha.1 has none.
fn checklist(current: &Version) -> Option<&'static str> {
    if (current.major, current.minor, current.patch) != (13, 0, 0) || current.pre.is_empty() {
        return None;
    }
    let pre = current.pre.as_str();
    match pre.strip_prefix("alpha.").map(str::parse::<u64>) {
        Some(Ok(2)) => Some(ALPHA2_CHECKLIST),
        Some(Ok(3)) => Some(ALPHA3_CHECKLIST),
        Some(Ok(4)) => Some(ALPHA4_CHECKLIST),
        Some(Ok(5)) => Some(ALPHA5_CHECKLIST),
        Some(Ok(number)) if number >= 6 => Some(ALPHA6_CHECKLIST),
        Some(_) => None,
        None if pre.starts_with("alpha") => None,
        None => Some(ALPHA6_CHECKLIST),
    }
}

/// What the plan says about the review locations a marker left out.
fn reviewed_note(scan: &SourceScan) -> Option<String> {
    let count = scan.reviewed.len();
    (count > 0).then(|| {
        format!(
            "{count} review location(s) marked `{}` left out",
            rules::REVIEWED_MARKER
        )
    })
}

/// The part of the guide the plan links: the checklist for the project's
/// version (or the `--from` version), or, for a v12 project, alpha.1, an
/// unknown version or `--all-rules`, the table of changes from 12.1.0 that
/// holds every finding's row.
pub(super) fn guide_url(selection: &RuleSelection) -> String {
    let checklist = selection
        .baseline()
        .filter(|_| !selection.all_rules)
        .and_then(checklist);
    match checklist {
        Some(anchor) => format!("{GUIDE}#{anchor}"),
        None => rules::MIGRATION_GUIDE_URL.to_string(),
    }
}

/// What the plan says about the rules it skipped, when it skipped any.
fn selection_note(selection: &RuleSelection) -> Option<String> {
    let baseline = selection.baseline()?;
    let source = match (&selection.from, &selection.current) {
        (None, _) => "the Rullst version this project uses".to_string(),
        (Some(_), Some(current)) => format!("passed with --from (this project uses {current})"),
        (Some(_), None) => "passed with --from".to_string(),
    };
    (selection.skipped_rules > 0).then(|| {
        format!(
            "introduced after {baseline}, {source}; earlier rules skipped: {} (findings hidden: {}); --all-rules applies every rule",
            selection.skipped_rules, selection.hidden_findings
        )
    })
}

/// The warning of a plan for a project that already uses the target
/// version without `--from`: the rules introduced in that version were
/// skipped with the earlier ones, so a plan without findings says nothing
/// about them.
pub(super) fn at_target_warning(target: &Version, selection: &RuleSelection) -> Option<String> {
    let current = selection.current.as_ref()?;
    if selection.from.is_some()
        || selection.skipped_rules == 0
        || current.cmp_precedence(target).is_ne()
    {
        return None;
    }
    let example = rules::release_before(current)
        .map(|release| format!(" (for example `--from {release}`)"))
        .unwrap_or_default();
    Some(format!(
        "this project already uses {current}, the target version, so the rules introduced in {current} and earlier did not run, and a plan without findings says nothing about them; to apply the rules introduced since the version you upgraded from, run `cargo rullst upgrade --dry-run --from <VERSION>` with that version{example}, or `--all-rules` for every rule"
    ))
}

/// A rule's findings in one file and every line they are at, ascending.
type FileRuleEntry<'a> = (&'a SourceFinding, Vec<usize>);

/// The human plan's entries: one per rule and file, listing every line the
/// rule matched there, ordered by file, first line and code. The JSON
/// document keeps one finding per location.
fn by_file_and_rule(findings: &[SourceFinding]) -> Vec<FileRuleEntry<'_>> {
    let mut groups: BTreeMap<(&Path, &str), FileRuleEntry<'_>> = BTreeMap::new();
    for finding in findings {
        groups
            .entry((finding.path.as_path(), finding.rule.code))
            .or_insert_with(|| (finding, Vec::new()))
            .1
            .push(finding.line);
    }
    let mut entries: Vec<_> = groups
        .into_values()
        .map(|(finding, mut lines)| {
            lines.sort_unstable();
            (finding, lines)
        })
        .collect();
    entries.sort_by(|(left, left_lines), (right, right_lines)| {
        left.path
            .cmp(&right.path)
            .then(left_lines.first().cmp(&right_lines.first()))
            .then(left.rule.code.cmp(right.rule.code))
    });
    entries
}

/// The notes of an entry's rule in its file, by line.
fn notes<'a>(findings: &'a [SourceFinding], entry: &SourceFinding) -> Vec<(usize, &'a str)> {
    let mut notes: Vec<_> = findings
        .iter()
        .filter(|finding| finding.path == entry.path && finding.rule.code == entry.rule.code)
        .filter_map(|finding| finding.note.as_deref().map(|note| (finding.line, note)))
        .collect();
    notes.sort_unstable();
    notes
}

/// `12, 40, 77`.
fn line_list(lines: &[usize]) -> String {
    lines
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
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
    if let Some(warning) = at_target_warning(target, &scan.selection) {
        lines.push(format!("  WARNING: {warning}"));
    }
    if let Some(note) = reviewed_note(scan) {
        lines.push(format!("  Reviewed: {note}"));
    }
    for (finding, at) in by_file_and_rule(&scan.findings) {
        lines.push(format!(
            "  {} {}:{} [{}] {}",
            finding.rule.kind.label(),
            relative_report_path(root, &finding.path),
            line_list(&at),
            finding.rule.code,
            finding.rule.message
        ));
        lines.push(format!("      migration-v13 row: {}", finding.rule.row));
        for (line, note) in notes(&scan.findings, finding) {
            lines.push(format!("      line {line}: {note}"));
        }
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
    if let Some(warning) = at_target_warning(target, &scan.selection) {
        report.push_str(&format!("**Warning:** {warning}.\n\n"));
    }
    if let Some(note) = reviewed_note(scan) {
        report.push_str(&format!("Reviewed: {note}.\n\n"));
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
    for (finding, at) in by_file_and_rule(&scan.findings) {
        report.push_str(&format!(
            "- **{}** `{}` {} {} (`{}`): {} — migration row “{}”\n",
            finding.rule.kind.label(),
            relative_report_path(root, &finding.path),
            if at.len() == 1 { "line" } else { "lines" },
            line_list(&at),
            finding.rule.code,
            finding.rule.message,
            finding.rule.row
        ));
        for (line, note) in notes(&scan.findings, finding) {
            report.push_str(&format!("  - line {line}: {note}\n"));
        }
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

/// The `rullst.upgrade-plan.v1` document, with one finding per location
/// (sorted by path, line and code). v13 extends it additively: each
/// finding keeps `path`, `line`, `code`, `severity` and `message` and adds
/// `kind`, `introduced_in`, `migration_row`, `migration_url` and `note`
/// (`null` unless the rule computed something for the location); the root adds
/// `rule_selection` (with `from_version` and `target_rules_skipped` since
/// 13.0.0-rc.1), and `finding_counts.reviewed` counts the review locations
/// a `rullst-upgrade: reviewed` marker left out.
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
                "note": finding.note,
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
            "reviewed": scan.reviewed.len(),
        },
        "rule_selection": {
            "current_version": scan.selection.current.as_ref().map(Version::to_string),
            "from_version": scan.selection.from.as_ref().map(Version::to_string),
            "all_rules": scan.selection.all_rules,
            "skipped_rules": scan.selection.skipped_rules,
            "hidden_findings": scan.selection.hidden_findings,
            "target_rules_skipped": at_target_warning(target, &scan.selection).is_some(),
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
