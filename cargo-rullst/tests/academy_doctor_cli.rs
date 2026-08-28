//! Process-level contract tests for the Academy diagnostic.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use serde_json::Value;
use std::fs;
use std::process::Command;

const REQUIREMENTS: [&str; 12] = [
    "authenticated_identity",
    "school_membership",
    "active_entitlement",
    "object_authorization",
    "tenant_isolation",
    "server_validated_assessment",
    "idempotent_score_events",
    "durable_automation",
    "durable_admin_audit",
    "safe_content_pipeline",
    "privacy_lifecycle",
    "distributed_abuse_control",
];

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rullst"));
    command.env("RULLST_DISABLE_UPDATE_CHECK", "1");
    command
}

#[test]
fn academy_doctor_process_fails_closed_without_evidence() {
    let output = command()
        .args(["academy:doctor", "--json"])
        .output()
        .expect("Academy diagnostic process");

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("diagnostic JSON");
    assert_eq!(report["certification"], false);
    assert_eq!(report["contract_satisfied"], false);
    assert_eq!(
        report["checks"].as_array().map(Vec::len),
        Some(REQUIREMENTS.len())
    );
    assert!(
        report["checks"]
            .as_array()
            .expect("checks")
            .iter()
            .all(|check| check["status"] == "NOT_EVALUATED")
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("ContractNotSatisfied"));
}

#[test]
fn academy_doctor_process_accepts_complete_declared_evidence_without_certifying() {
    let checks = REQUIREMENTS
        .iter()
        .map(|requirement| {
            serde_json::json!({
                "requirement": requirement,
                "status": "PASS",
                "evidence": [format!("test:{requirement}")]
            })
        })
        .collect::<Vec<_>>();
    let evidence = serde_json::json!({
        "schema_version": "rullst.academy-evidence.v1",
        "checks": checks
    });
    let evidence_path = std::env::temp_dir().join(format!(
        "rullst-academy-evidence-{}.json",
        rand::random::<u64>()
    ));
    fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&evidence).expect("evidence JSON"),
    )
    .expect("temporary evidence file");

    let output = command()
        .args(["academy:doctor", "--evidence"])
        .arg(&evidence_path)
        .arg("--json")
        .output()
        .expect("Academy diagnostic process");
    let _ = fs::remove_file(&evidence_path);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("diagnostic JSON");
    assert_eq!(report["certification"], false);
    assert_eq!(report["contract_satisfied"], true);
    assert!(
        report["checks"]
            .as_array()
            .expect("checks")
            .iter()
            .all(|check| check["status"] == "PASS")
    );
}
