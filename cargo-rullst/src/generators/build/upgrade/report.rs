//! Human, Markdown and `rullst.upgrade-plan.v1` renderings of a plan.

use super::manifest::ManifestUpgradePlan;
use super::rules::{self, FindingKind, SourceScan};
use super::relative_report_path;
use colored::Colorize;
use semver::Version;
use std::path::Path;

/// The plan as plain text (no colour), shared by the CLI and the assistant.
pub(super) fn plan_text(
    root: &Path,
    target: &Version,
    plans: &[ManifestUpgradePlan],
    scan: &SourceScan,
) -> String {
    let mut lines = vec![format!("Project: {}", root.display())];
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
    if !scan.findings.is_empty() {
        lines.push(format!("Migration guide: {}", rules::MIGRATION_GUIDE_URL));
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
        root.display()
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
/// `kind`, `migration_row` and `migration_url`.
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
        "schema_version": "rullst.upgrade-plan.v1",
        "rule_catalog": rules::RULE_CATALOG_VERSION,
        "target": target.to_string(),
        "manifests": manifests,
        "source_findings": findings,
        "finding_counts": {
            "must_change": scan.count(FindingKind::MustChange),
            "review": scan.count(FindingKind::Review),
        },
        "unscanned_sources": unscanned,
        "migration_guide": rules::MIGRATION_GUIDE_URL,
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
