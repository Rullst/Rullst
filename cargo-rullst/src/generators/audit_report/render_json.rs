//! The `rullst.cli-audit-report.v1` JSON document.
//!
//! Key names and nesting are a documented contract (`cli_reference.md`); a
//! new key needs a new schema version. Strings come from sanitized findings,
//! so the document carries no secret value and no ANSI sequence.

use serde_json::{Value, json};

use super::catalog::{
    ASVS_NAME, ASVS_SOURCE, ASVS_VERSION, MAPPING_NOTE, NOTICE, PERSONAL_DATA, WCAG_VERSION,
    level1_coverage, not_evaluated,
};
use super::model::{Check, Finding, Report};

pub(crate) const SCHEMA_VERSION: &str = "rullst.cli-audit-report.v1";

fn count(status: &crate::generators::audit_compliance::EvidenceStatus) -> Value {
    use crate::generators::audit_compliance::EvidenceStatus as Status;
    match status {
        Status::Findings(count) | Status::Generated(count) | Status::Observed(count) => {
            json!(count)
        }
        _ => Value::Null,
    }
}

fn exceptions(status: &crate::generators::audit_compliance::EvidenceStatus) -> Vec<String> {
    match status {
        crate::generators::audit_compliance::EvidenceStatus::NoFindingsOutsideExceptions(ids) => {
            ids.clone()
        }
        _ => Vec::new(),
    }
}

fn finding(finding: &Finding) -> Value {
    json!({
        "file": finding.file,
        "line": finding.line,
        "message": finding.message,
        "preview": finding.preview,
    })
}

fn check(check: &Check) -> Value {
    json!({
        "id": check.spec.id,
        "group": check.spec.group.id(),
        "title": check.spec.title,
        "status": check.status.id(),
        "count": count(&check.status),
        "exceptions": exceptions(&check.status),
        "detail": check.detail,
        "fix": check.spec.fix,
        "docs": check.spec.docs(),
        "asvs": check.spec.asvs.iter().map(|reference| json!({
            "id": reference.id,
            "section": reference.section,
            "level": reference.level,
        })).collect::<Vec<_>>(),
        "asvs_chapter": check.spec.chapter,
        "wcag": check.spec.wcag,
        "findings": check.findings.iter().map(finding).collect::<Vec<_>>(),
    })
}

pub(crate) fn render(report: &Report) -> Result<String, serde_json::Error> {
    let (mapped, total) = level1_coverage(report);
    let personal = &report.personal_data;
    let document = json!({
        "schema_version": SCHEMA_VERSION,
        "generated_at": report.generated_at,
        "tool": { "name": "cargo-rullst", "version": env!("CARGO_PKG_VERSION") },
        "notice": NOTICE,
        "standard": {
            "name": ASVS_NAME,
            "version": ASVS_VERSION,
            "level": 1,
            "source": ASVS_SOURCE,
            "level1_requirements": total,
            "level1_mapped": mapped,
            "mapping_note": MAPPING_NOTE,
        },
        "accessibility_standard": WCAG_VERSION,
        "exit_non_zero": report.fails(),
        "checks": report.checks.iter().map(check).collect::<Vec<_>>(),
        "personal_data": {
            "status": personal.status.id(),
            "count": count(&personal.status),
            "review": personal.review_count(),
            "detail": personal.detail,
            "docs": PERSONAL_DATA.docs(),
            "asvs_chapter": PERSONAL_DATA.chapter,
            "fields": personal.fields.iter().map(|field| json!({
                "model": field.model,
                "field": field.field,
                "file": field.file,
                "line": field.line,
                "classification": field.classification,
                "encrypted": field.encrypted,
            })).collect::<Vec<_>>(),
        },
        "not_evaluated": not_evaluated(report).iter().map(|chapter| json!({
            "chapter": chapter.chapter,
            "name": chapter.name,
            "status": "not_evaluated",
            "whole_chapter": chapter.whole_chapter,
            "requirements": chapter.requirements,
        })).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&document)
}
