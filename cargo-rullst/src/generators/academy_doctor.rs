//! Fail-closed Academy production-boundary diagnostics.

use rullst_core::{
    AcademyCheckStatus, AcademyPresetError, AcademyProductionRequirement, ProductionPreset,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const EVIDENCE_SCHEMA: &str = "rullst.academy-evidence.v1";
const DIAGNOSTIC_SCHEMA: &str = "rullst.academy-diagnostic.v1";

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AcademyDoctorError {
    #[error("failed to read Academy evidence file {path}: {source}")]
    ReadEvidence {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid Academy evidence JSON in {path}: {source}")]
    InvalidEvidence {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported Academy evidence schema {0:?}; expected {EVIDENCE_SCHEMA:?}")]
    UnsupportedSchema(String),
    #[error("Academy requirement {0} declared PASS without non-empty evidence")]
    PassWithoutEvidence(&'static str),
    #[error(transparent)]
    Preset(#[from] AcademyPresetError),
    #[error("failed to serialize the Academy diagnostic: {0}")]
    Serialize(serde_json::Error),
    #[error("Academy production-boundary contract is not satisfied")]
    ContractNotSatisfied,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceFile {
    schema_version: String,
    #[serde(default)]
    checks: Vec<EvidenceCheck>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceCheck {
    requirement: RequirementName,
    status: DeclaredStatus,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequirementName {
    AuthenticatedIdentity,
    SchoolMembership,
    ActiveEntitlement,
    ObjectAuthorization,
    TenantIsolation,
    ServerValidatedAssessment,
    IdempotentScoreEvents,
    DurableAutomation,
    DurableAdminAudit,
    SafeContentPipeline,
    PrivacyLifecycle,
    DistributedAbuseControl,
}

impl RequirementName {
    const fn as_str(self) -> &'static str {
        match self {
            Self::AuthenticatedIdentity => "authenticated_identity",
            Self::SchoolMembership => "school_membership",
            Self::ActiveEntitlement => "active_entitlement",
            Self::ObjectAuthorization => "object_authorization",
            Self::TenantIsolation => "tenant_isolation",
            Self::ServerValidatedAssessment => "server_validated_assessment",
            Self::IdempotentScoreEvents => "idempotent_score_events",
            Self::DurableAutomation => "durable_automation",
            Self::DurableAdminAudit => "durable_admin_audit",
            Self::SafeContentPipeline => "safe_content_pipeline",
            Self::PrivacyLifecycle => "privacy_lifecycle",
            Self::DistributedAbuseControl => "distributed_abuse_control",
        }
    }

    const fn requirement(self) -> AcademyProductionRequirement {
        match self {
            Self::AuthenticatedIdentity => AcademyProductionRequirement::AuthenticatedIdentity,
            Self::SchoolMembership => AcademyProductionRequirement::SchoolMembership,
            Self::ActiveEntitlement => AcademyProductionRequirement::ActiveEntitlement,
            Self::ObjectAuthorization => AcademyProductionRequirement::ObjectAuthorization,
            Self::TenantIsolation => AcademyProductionRequirement::TenantIsolation,
            Self::ServerValidatedAssessment => {
                AcademyProductionRequirement::ServerValidatedAssessment
            }
            Self::IdempotentScoreEvents => AcademyProductionRequirement::IdempotentScoreEvents,
            Self::DurableAutomation => AcademyProductionRequirement::DurableAutomation,
            Self::DurableAdminAudit => AcademyProductionRequirement::DurableAdminAudit,
            Self::SafeContentPipeline => AcademyProductionRequirement::SafeContentPipeline,
            Self::PrivacyLifecycle => AcademyProductionRequirement::PrivacyLifecycle,
            Self::DistributedAbuseControl => AcademyProductionRequirement::DistributedAbuseControl,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum DeclaredStatus {
    Pass,
    Fail,
    Skipped,
    NotEvaluated,
}

impl DeclaredStatus {
    const fn status(self) -> AcademyCheckStatus {
        match self {
            Self::Pass => AcademyCheckStatus::Pass,
            Self::Fail => AcademyCheckStatus::Fail,
            Self::Skipped => AcademyCheckStatus::Skipped,
            Self::NotEvaluated => AcademyCheckStatus::NotEvaluated,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct AcademyDiagnostic {
    schema_version: &'static str,
    certification: bool,
    contract_satisfied: bool,
    checks: Vec<DiagnosticCheck>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct DiagnosticCheck {
    requirement: &'static str,
    status: &'static str,
    declared_evidence: Vec<String>,
}

fn evaluate(evidence: EvidenceFile) -> Result<AcademyDiagnostic, AcademyDoctorError> {
    if evidence.schema_version != EVIDENCE_SCHEMA {
        return Err(AcademyDoctorError::UnsupportedSchema(
            evidence.schema_version,
        ));
    }

    for check in &evidence.checks {
        if check.status == DeclaredStatus::Pass
            && !check.evidence.iter().any(|item| !item.trim().is_empty())
        {
            return Err(AcademyDoctorError::PassWithoutEvidence(
                check.requirement.as_str(),
            ));
        }
    }

    let observations = evidence
        .checks
        .iter()
        .map(|check| (check.requirement.requirement(), check.status.status()))
        .collect::<Vec<_>>();
    let preset = ProductionPreset::academy();
    let assessments = preset.assess(observations.iter().copied())?;
    let contract_satisfied = preset.validate(observations).is_ok();
    let checks = assessments
        .into_iter()
        .map(|assessment| {
            let requirement = assessment.requirement();
            let declared_evidence = evidence
                .checks
                .iter()
                .find(|check| check.requirement.requirement() == requirement)
                .map(|check| {
                    check
                        .evidence
                        .iter()
                        .filter_map(|item| {
                            let item = item.trim();
                            (!item.is_empty()).then(|| item.to_string())
                        })
                        .collect()
                })
                .unwrap_or_default();
            DiagnosticCheck {
                requirement: requirement.as_str(),
                status: assessment.status().as_str(),
                declared_evidence,
            }
        })
        .collect();

    Ok(AcademyDiagnostic {
        schema_version: DIAGNOSTIC_SCHEMA,
        certification: false,
        contract_satisfied,
        checks,
    })
}

/// Runs the Academy boundary diagnostic and exits unsuccessfully unless every requirement passes.
pub fn run_academy_doctor(
    evidence_path: Option<&Path>,
    json: bool,
) -> Result<(), AcademyDoctorError> {
    let evidence = if let Some(path) = evidence_path {
        let contents =
            fs::read_to_string(path).map_err(|source| AcademyDoctorError::ReadEvidence {
                path: path.to_path_buf(),
                source,
            })?;
        serde_json::from_str(&contents).map_err(|source| AcademyDoctorError::InvalidEvidence {
            path: path.to_path_buf(),
            source,
        })?
    } else {
        EvidenceFile {
            schema_version: EVIDENCE_SCHEMA.to_string(),
            checks: Vec::new(),
        }
    };
    let diagnostic = evaluate(evidence)?;

    if json {
        let output =
            serde_json::to_string_pretty(&diagnostic).map_err(AcademyDoctorError::Serialize)?;
        println!("{output}");
    } else {
        println!("Academy production-boundary diagnostic");
        println!("certification: false");
        for check in &diagnostic.checks {
            println!("{}: {}", check.requirement, check.status);
        }
        println!("contract_satisfied: {}", diagnostic.contract_satisfied);
    }

    if diagnostic.contract_satisfied {
        Ok(())
    } else {
        Err(AcademyDoctorError::ContractNotSatisfied)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AcademyDoctorError, DeclaredStatus, EVIDENCE_SCHEMA, EvidenceCheck, EvidenceFile,
        RequirementName, evaluate,
    };

    #[test]
    fn missing_observations_are_not_evaluated_and_fail_closed() {
        let diagnostic = evaluate(EvidenceFile {
            schema_version: EVIDENCE_SCHEMA.to_string(),
            checks: Vec::new(),
        })
        .expect("empty evidence must normalize");

        assert!(!diagnostic.certification);
        assert!(!diagnostic.contract_satisfied);
        assert_eq!(diagnostic.checks.len(), 12);
        assert!(
            diagnostic
                .checks
                .iter()
                .all(|check| check.status == "NOT_EVALUATED")
        );
    }

    #[test]
    fn explicit_evidenced_passes_satisfy_the_contract_without_certification() {
        let checks = [
            RequirementName::AuthenticatedIdentity,
            RequirementName::SchoolMembership,
            RequirementName::ActiveEntitlement,
            RequirementName::ObjectAuthorization,
            RequirementName::TenantIsolation,
            RequirementName::ServerValidatedAssessment,
            RequirementName::IdempotentScoreEvents,
            RequirementName::DurableAutomation,
            RequirementName::DurableAdminAudit,
            RequirementName::SafeContentPipeline,
            RequirementName::PrivacyLifecycle,
            RequirementName::DistributedAbuseControl,
        ]
        .into_iter()
        .map(|requirement| EvidenceCheck {
            requirement,
            status: DeclaredStatus::Pass,
            evidence: vec![format!("test:{}", requirement.as_str())],
        })
        .collect();
        let diagnostic = evaluate(EvidenceFile {
            schema_version: EVIDENCE_SCHEMA.to_string(),
            checks,
        })
        .expect("complete evidence must normalize");

        assert!(diagnostic.contract_satisfied);
        assert!(!diagnostic.certification);
        assert!(diagnostic.checks.iter().all(|check| check.status == "PASS"));
    }

    #[test]
    fn pass_without_evidence_is_rejected() {
        let result = evaluate(EvidenceFile {
            schema_version: EVIDENCE_SCHEMA.to_string(),
            checks: vec![EvidenceCheck {
                requirement: RequirementName::AuthenticatedIdentity,
                status: DeclaredStatus::Pass,
                evidence: Vec::new(),
            }],
        });

        assert!(matches!(
            result,
            Err(AcademyDoctorError::PassWithoutEvidence(_))
        ));
    }
}
