use std::path::Path;

use crate::generators::output_guard::write_output;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceStatus {
    NoFindings,
    NoFindingsOutsideExceptions(Vec<String>),
    Findings(usize),
    Generated(usize),
    Observed(usize),
    NotChecked(&'static str),
    Error(String),
}

impl EvidenceStatus {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::NoFindings => "NO FINDINGS",
            Self::NoFindingsOutsideExceptions(_) => "NO FINDINGS OUTSIDE EXCEPTIONS",
            Self::Findings(_) => "FINDINGS",
            Self::Generated(_) => "GENERATED",
            Self::Observed(_) => "OBSERVED",
            Self::NotChecked(_) => "NOT CHECKED",
            Self::Error(_) => "ERROR",
        }
    }

    /// The stable machine-readable name used by the JSON documents.
    pub(crate) fn id(&self) -> &'static str {
        match self {
            Self::NoFindings => "no_findings",
            Self::NoFindingsOutsideExceptions(_) => "no_findings_outside_exceptions",
            Self::Findings(_) => "findings",
            Self::Generated(_) => "generated",
            Self::Observed(_) => "observed",
            Self::NotChecked(_) => "not_checked",
            Self::Error(_) => "error",
        }
    }

    /// Whether this status makes the audit exit non-zero.
    pub(crate) fn fails(&self) -> bool {
        matches!(self, Self::Findings(_) | Self::Error(_))
    }

    pub(crate) fn detail(&self) -> String {
        match self {
            Self::NoFindings => "The named check ran and reported no findings.".to_string(),
            Self::NoFindingsOutsideExceptions(advisories) => format!(
                "The check completed with explicit governed exception(s): {}. These advisories remain findings and require separate review.",
                advisories.join(", ")
            ),
            Self::Findings(count) => format!("{count} finding(s) reported."),
            Self::Generated(count) => format!("Artifact generated with {count} component(s)."),
            Self::Observed(count) => format!("The bounded scan recorded {count} observation(s)."),
            Self::NotChecked(reason) => (*reason).to_string(),
            Self::Error(error) => format!("Check failed to complete: {error}"),
        }
    }
}

/// One check of the `audit --json` summary.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct CheckSummary {
    pub id: &'static str,
    pub status: &'static str,
    pub count: Option<usize>,
    pub exceptions: Vec<String>,
    pub detail: String,
}

/// The versioned `audit --json` document (`rullst.cli-audit.v1`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct AuditSummary {
    pub schema_version: &'static str,
    pub issues_found: usize,
    pub checks: Vec<CheckSummary>,
}

fn check_summary(id: &'static str, status: &EvidenceStatus) -> CheckSummary {
    let (count, exceptions) = match status {
        EvidenceStatus::NoFindingsOutsideExceptions(advisories) => (None, advisories.clone()),
        EvidenceStatus::Findings(count)
        | EvidenceStatus::Generated(count)
        | EvidenceStatus::Observed(count) => (Some(*count), Vec::new()),
        _ => (None, Vec::new()),
    };
    CheckSummary {
        id,
        status: status.id(),
        count,
        exceptions,
        detail: crate::ui::error_report::sanitize(&status.detail()),
    }
}

/// The JSON summary of one audit run; details never carry secret values.
pub(crate) fn summary(evidence: &ComplianceEvidence, issues_found: usize) -> AuditSummary {
    AuditSummary {
        schema_version: "rullst.cli-audit.v1",
        issues_found,
        checks: vec![
            check_summary("secret_scan", &evidence.secret_scan),
            check_summary("dependency_audit", &evidence.dependency_audit),
            check_summary("unsafe_scan", &evidence.unsafe_scan),
            check_summary("idor_scan", &evidence.idor_scan),
            check_summary("sbom", &evidence.sbom),
            check_summary("network_scan", &evidence.network_scan),
        ],
    }
}

#[derive(Debug, Clone)]
pub struct ComplianceEvidence {
    pub secret_scan: EvidenceStatus,
    pub dependency_audit: EvidenceStatus,
    pub unsafe_scan: EvidenceStatus,
    pub idor_scan: EvidenceStatus,
    pub sbom: EvidenceStatus,
    pub network_scan: EvidenceStatus,
}

fn row(report: &mut String, check: &str, status: &EvidenceStatus) {
    report.push_str(&format!(
        "| {check} | **{}** | {} |\n",
        status.label(),
        status.detail().replace('|', "\\|").replace('\n', " ")
    ));
}

/// Writes the evidence report, replacing a previous regular file but never
/// following a symlink (the audited checkout may be untrusted).
pub fn write_compliance_report(
    output: &Path,
    evidence: &ComplianceEvidence,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut report = String::new();
    report.push_str("# Rullst Security Evidence Report\n\n");
    report.push_str("> Generated by cargo rullst audit --compliance. This is a record of checks actually executed, not an OWASP, SOC 2, ISO 27001, FedRAMP, or regulatory certification.\n\n");
    report.push_str(&format!(
        "- Generated at: {}\n",
        chrono::Utc::now().to_rfc3339()
    ));
    report.push_str("- Scope: current project source tree and local Cargo metadata.\n\n");
    report.push_str("## Executed evidence\n\n");
    report.push_str("| Check | Result | Evidence boundary |\n");
    report.push_str("| :--- | :--- | :--- |\n");
    row(
        &mut report,
        "Local secret-strength heuristic",
        &evidence.secret_scan,
    );
    row(&mut report, "cargo audit", &evidence.dependency_audit);
    row(
        &mut report,
        "Project-source unsafe scan",
        &evidence.unsafe_scan,
    );
    row(
        &mut report,
        "Parameterized-route IDOR heuristic",
        &evidence.idor_scan,
    );
    row(&mut report, "CycloneDX SBOM generation", &evidence.sbom);
    row(
        &mut report,
        "Local network-surface scan",
        &evidence.network_scan,
    );

    report.push_str("\n## Controls not established by this command\n\n");
    report.push_str("| Control family | Status | Reason |\n");
    report.push_str("| :--- | :--- | :--- |\n");
    report.push_str("| OWASP Top 10 conformance | **NOT EVALUATED** | Requires application-specific design review, dynamic testing, and deployment evidence. |\n");
    report.push_str("| Cryptographic key management / vault | **NOT EVALUATED** | Source heuristics do not prove algorithms, key custody, rotation, or data recovery. |\n");
    report.push_str("| CSP / secure-header deployment | **NOT EVALUATED** | Requires response-level tests against the deployed application. |\n");
    report.push_str("| Authentication, MFA, and brute-force controls | **NOT EVALUATED** | Presence of framework modules does not prove route composition or policy. |\n");
    report.push_str("| TLS configuration | **NOT EVALUATED** | Requires inspection of the actual ingress and certificate configuration. |\n");
    report.push_str("| SOC 2 / ISO 27001 / FedRAMP readiness | **NOT EVALUATED** | Organizational controls and independent audit evidence are outside this static command. |\n\n");
    report.push_str("NO FINDINGS means only that the named bounded check completed without a finding. It is not a certification result.\n");
    report.push_str("NO FINDINGS OUTSIDE EXCEPTIONS means explicit advisory exceptions were supplied by the caller; those advisories remain unresolved and require separate governance.\n");

    write_output(output, report.as_bytes(), true)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn report_never_fabricates_compliance_passes() {
        let output =
            std::env::temp_dir().join(format!("rullst-compliance-{}.md", rand::random::<u64>()));
        let evidence = ComplianceEvidence {
            secret_scan: EvidenceStatus::NoFindings,
            dependency_audit: EvidenceStatus::NotChecked("cargo-audit unavailable"),
            unsafe_scan: EvidenceStatus::Findings(2),
            idor_scan: EvidenceStatus::NoFindings,
            sbom: EvidenceStatus::NotChecked("SBOM not requested"),
            network_scan: EvidenceStatus::NotChecked("network scan not requested"),
        };
        write_compliance_report(&output, &evidence).expect("evidence report");
        let report = fs::read_to_string(&output).expect("evidence contents");
        assert!(!report.contains("✅ PASS"));
        assert!(!report.contains("FedRAMP Ready"));
        assert!(!report.contains("Vault AES-256"));
        assert!(report.contains("NOT EVALUATED"));
        assert!(!report.contains("PASS"));
        fs::remove_file(output).expect("temporary report cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn report_is_never_written_through_a_committed_symlink() {
        let directory = tempfile::tempdir().expect("temporary checkout");
        let victim = directory.path().join("authorized_keys");
        fs::write(&victim, "ssh-ed25519 fixture").expect("victim file");
        let output = directory.path().join("SECURITY_COMPLIANCE.md");
        std::os::unix::fs::symlink(&victim, &output).expect("committed symlink");
        let evidence = ComplianceEvidence {
            secret_scan: EvidenceStatus::NoFindings,
            dependency_audit: EvidenceStatus::NoFindings,
            unsafe_scan: EvidenceStatus::NoFindings,
            idor_scan: EvidenceStatus::NoFindings,
            sbom: EvidenceStatus::NotChecked("SBOM not requested"),
            network_scan: EvidenceStatus::NotChecked("network scan not requested"),
        };

        assert!(write_compliance_report(&output, &evidence).is_err());
        assert_eq!(
            fs::read_to_string(&victim).expect("victim contents"),
            "ssh-ed25519 fixture"
        );
    }

    #[test]
    fn the_json_summary_has_the_documented_shape() {
        let evidence = ComplianceEvidence {
            secret_scan: EvidenceStatus::Findings(2),
            dependency_audit: EvidenceStatus::NoFindingsOutsideExceptions(vec![
                "RUSTSEC-2099-0001".to_string(),
            ]),
            unsafe_scan: EvidenceStatus::NoFindings,
            idor_scan: EvidenceStatus::Error("walk failed for postgres://u:hunter2@db".to_string()),
            sbom: EvidenceStatus::NotChecked("SBOM generation was not requested"),
            network_scan: EvidenceStatus::Observed(1),
        };
        let value = serde_json::to_value(summary(&evidence, 3)).expect("serializable");
        assert_eq!(value["schema_version"], "rullst.cli-audit.v1");
        assert_eq!(value["issues_found"], 3);
        let checks = value["checks"].as_array().expect("checks");
        let ids: Vec<_> = checks.iter().map(|check| check["id"].clone()).collect();
        assert_eq!(
            ids,
            [
                "secret_scan",
                "dependency_audit",
                "unsafe_scan",
                "idor_scan",
                "sbom",
                "network_scan"
            ]
        );
        assert_eq!(checks[0]["status"], "findings");
        assert_eq!(checks[0]["count"], 2);
        assert_eq!(checks[1]["status"], "no_findings_outside_exceptions");
        assert_eq!(checks[1]["exceptions"][0], "RUSTSEC-2099-0001");
        assert_eq!(checks[3]["status"], "error");
        assert!(
            !checks[3]["detail"]
                .as_str()
                .unwrap_or_default()
                .contains("hunter2")
        );
        assert_eq!(checks[4]["status"], "not_checked");
        assert!(checks[4]["count"].is_null());
    }

    #[test]
    fn report_preserves_governed_advisory_exceptions() {
        let output = std::env::temp_dir().join(format!(
            "rullst-compliance-exceptions-{}.md",
            rand::random::<u64>()
        ));
        let evidence = ComplianceEvidence {
            secret_scan: EvidenceStatus::NoFindings,
            dependency_audit: EvidenceStatus::NoFindingsOutsideExceptions(vec![
                "RUSTSEC-2099-0001".to_string(),
            ]),
            unsafe_scan: EvidenceStatus::NoFindings,
            idor_scan: EvidenceStatus::NoFindings,
            sbom: EvidenceStatus::NotChecked("SBOM not requested"),
            network_scan: EvidenceStatus::NotChecked("network scan not requested"),
        };
        write_compliance_report(&output, &evidence).expect("evidence report");
        let report = fs::read_to_string(&output).expect("evidence contents");
        assert!(report.contains("NO FINDINGS OUTSIDE EXCEPTIONS"));
        assert!(report.contains("RUSTSEC-2099-0001"));
        assert!(report.contains("remain unresolved"));
        fs::remove_file(output).expect("temporary report cleanup");
    }
}
