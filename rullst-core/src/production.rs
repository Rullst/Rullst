//! Canonical production middleware ordering contract.
//!
//! The framework owns the transport and baseline-security stages. Session,
//! authentication, tenant resolution and authorization remain application
//! layers because their state and policies are domain-specific.

/// A stage in the canonical inbound production middleware chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProductionMiddlewareStage {
    /// Accept forwarded identity only through an explicitly trusted proxy policy.
    TrustedProxy,
    /// Reject request bodies above the route/application limit before buffering.
    BodyLimit,
    /// Accept or generate a validated request correlation identifier.
    RequestId,
    /// Start request tracing after correlation identity exists.
    Tracing,
    /// Wrap every inner response with the strict secure-header baseline.
    SecureHeaders,
    /// Apply an explicit origin allowlist; an empty list authorizes no cross-origin caller.
    Cors,
    /// Inspect supported, bounded request data with the configured WAF/RASP policy.
    WafRasp,
    /// Validate browser-originated state-changing requests.
    Csrf,
    /// Resolve and cryptographically validate the application session.
    Session,
    /// Construct the authenticated subject from validated credentials.
    Authentication,
    /// Resolve tenant membership from the authenticated subject.
    Tenant,
    /// Enforce role, permission and object ownership.
    Authorization,
    /// Rate limit by authenticated identity when present, otherwise by direct peer.
    RateLimit,
    /// Dispatch the application handler only after every applicable guard.
    Handler,
}

/// Typed, stable description of the v12 production middleware preset.
///
/// This type documents the outer-to-inner request order. It deliberately does
/// not fabricate generic authentication or authorization: applications mount
/// those domain layers in the slots declared here.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProductionPreset;

impl ProductionPreset {
    /// Canonical outer-to-inner request order for production applications.
    pub const MIDDLEWARE_ORDER: &'static [ProductionMiddlewareStage] = &[
        ProductionMiddlewareStage::TrustedProxy,
        ProductionMiddlewareStage::BodyLimit,
        ProductionMiddlewareStage::RequestId,
        ProductionMiddlewareStage::Tracing,
        ProductionMiddlewareStage::SecureHeaders,
        ProductionMiddlewareStage::Cors,
        ProductionMiddlewareStage::WafRasp,
        ProductionMiddlewareStage::Csrf,
        ProductionMiddlewareStage::Session,
        ProductionMiddlewareStage::Authentication,
        ProductionMiddlewareStage::Tenant,
        ProductionMiddlewareStage::Authorization,
        ProductionMiddlewareStage::RateLimit,
        ProductionMiddlewareStage::Handler,
    ];

    /// Returns the canonical outer-to-inner middleware order.
    pub const fn middleware_order() -> &'static [ProductionMiddlewareStage] {
        Self::MIDDLEWARE_ORDER
    }

    /// Creates the fail-closed Academy boundary assessment.
    ///
    /// Every Academy-specific requirement starts as [`AcademyCheckStatus::NotEvaluated`].
    /// Applications may attach observations from their own topology and call
    /// [`AcademyProductionPreset::validate`]; validation succeeds only when every requirement
    /// has exactly one explicit [`AcademyCheckStatus::Pass`]. This is an integration contract,
    /// not an automatic security certification.
    pub const fn academy() -> AcademyProductionPreset {
        AcademyProductionPreset
    }
}

/// An Academy domain boundary that must be evaluated by the deployed application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AcademyProductionRequirement {
    /// Identity originates from a validated server-side session or reviewed gateway.
    AuthenticatedIdentity,
    /// The subject has current membership in the selected school/tenant.
    SchoolMembership,
    /// Protected content is gated by a current server-side entitlement.
    ActiveEntitlement,
    /// Object ownership/role checks run before data or side effects are exposed.
    ObjectAuthorization,
    /// Database, cache, queue, storage, search and telemetry are tenant-isolated.
    TenantIsolation,
    /// Assessments and grades are validated or calculated authoritatively on the server.
    ServerValidatedAssessment,
    /// Score events are authenticated, versioned, bounded and idempotent.
    IdempotentScoreEvents,
    /// Domain events use a durable outbox and idempotent automation handlers.
    DurableAutomation,
    /// Administrative and grade/score mutations have a durable actor-bound audit trail.
    DurableAdminAudit,
    /// Uploads and active content pass type-specific quarantine/scanning policy.
    SafeContentPipeline,
    /// Consent, minimization, retention, export and deletion have been evaluated.
    PrivacyLifecycle,
    /// Abuse controls cover the deployment's distributed identity/origin boundary.
    DistributedAbuseControl,
}

impl AcademyProductionRequirement {
    /// Complete requirement inventory used by the fail-closed assessment.
    pub const ALL: &'static [Self] = &[
        Self::AuthenticatedIdentity,
        Self::SchoolMembership,
        Self::ActiveEntitlement,
        Self::ObjectAuthorization,
        Self::TenantIsolation,
        Self::ServerValidatedAssessment,
        Self::IdempotentScoreEvents,
        Self::DurableAutomation,
        Self::DurableAdminAudit,
        Self::SafeContentPipeline,
        Self::PrivacyLifecycle,
        Self::DistributedAbuseControl,
    ];

    /// Stable snake-case name used by diagnostics and evidence files.
    pub const fn as_str(self) -> &'static str {
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
}

/// Evidence status for one Academy production boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AcademyCheckStatus {
    /// A concrete check ran and passed within its declared scope.
    Pass,
    /// A concrete check ran and failed.
    Fail,
    /// The check was deliberately skipped with an application-owned justification.
    Skipped,
    /// No reliable check or evidence was supplied.
    NotEvaluated,
}

impl AcademyCheckStatus {
    /// Stable uppercase diagnostic label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Skipped => "SKIPPED",
            Self::NotEvaluated => "NOT_EVALUATED",
        }
    }
}

/// One normalized Academy boundary assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AcademyBoundaryAssessment {
    requirement: AcademyProductionRequirement,
    status: AcademyCheckStatus,
}

impl AcademyBoundaryAssessment {
    /// Assessed requirement.
    pub const fn requirement(&self) -> AcademyProductionRequirement {
        self.requirement
    }

    /// Normalized status, defaulting to `NOT_EVALUATED` when no observation was supplied.
    pub const fn status(&self) -> AcademyCheckStatus {
        self.status
    }
}

/// Fail-closed Academy preset validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AcademyPresetError {
    /// The caller supplied conflicting or duplicate evidence for one requirement.
    #[error("Academy requirement {0:?} was supplied more than once")]
    DuplicateRequirement(AcademyProductionRequirement),
    /// A requirement was absent, skipped or failed rather than explicitly passing.
    #[error("Academy requirement {requirement:?} is {status:?}, not PASS")]
    RequirementNotPassed {
        /// Requirement that prevented validation.
        requirement: AcademyProductionRequirement,
        /// Observed or defaulted status.
        status: AcademyCheckStatus,
    },
}

/// Fail-closed Academy domain assessment layered on the canonical production order.
#[derive(Debug, Clone, Copy, Default)]
pub struct AcademyProductionPreset;

impl AcademyProductionPreset {
    /// Uses the same outer-to-inner HTTP order as the base production preset.
    pub const fn middleware_order(&self) -> &'static [ProductionMiddlewareStage] {
        ProductionPreset::middleware_order()
    }

    /// Normalizes application observations against the complete requirement inventory.
    ///
    /// Missing observations are emitted as `NOT_EVALUATED`; duplicate observations are rejected.
    pub fn assess(
        &self,
        observations: impl IntoIterator<Item = (AcademyProductionRequirement, AcademyCheckStatus)>,
    ) -> Result<Vec<AcademyBoundaryAssessment>, AcademyPresetError> {
        let observations = observations.into_iter().collect::<Vec<_>>();
        for (index, (requirement, _)) in observations.iter().enumerate() {
            if observations[index + 1..]
                .iter()
                .any(|(candidate, _)| candidate == requirement)
            {
                return Err(AcademyPresetError::DuplicateRequirement(*requirement));
            }
        }

        Ok(AcademyProductionRequirement::ALL
            .iter()
            .map(|requirement| AcademyBoundaryAssessment {
                requirement: *requirement,
                status: observations
                    .iter()
                    .find_map(|(candidate, status)| (candidate == requirement).then_some(*status))
                    .unwrap_or(AcademyCheckStatus::NotEvaluated),
            })
            .collect())
    }

    /// Validates only an explicit all-`PASS` assessment.
    pub fn validate(
        &self,
        observations: impl IntoIterator<Item = (AcademyProductionRequirement, AcademyCheckStatus)>,
    ) -> Result<(), AcademyPresetError> {
        for assessment in self.assess(observations)? {
            if assessment.status != AcademyCheckStatus::Pass {
                return Err(AcademyPresetError::RequirementNotPassed {
                    requirement: assessment.requirement,
                    status: assessment.status,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AcademyCheckStatus, AcademyPresetError, AcademyProductionRequirement,
        ProductionMiddlewareStage, ProductionPreset,
    };
    use std::collections::HashSet;

    #[test]
    fn production_order_is_complete_unique_and_ends_at_the_handler() {
        let order = ProductionPreset::middleware_order();
        let unique = order.iter().copied().collect::<HashSet<_>>();

        assert_eq!(order.len(), 14);
        assert_eq!(unique.len(), order.len());
        assert_eq!(
            order.first(),
            Some(&ProductionMiddlewareStage::TrustedProxy)
        );
        assert_eq!(order.last(), Some(&ProductionMiddlewareStage::Handler));
    }

    #[test]
    fn identity_is_validated_before_tenant_authorization_and_rate_limit() {
        let order = ProductionPreset::middleware_order();
        let expected_identity_order = [
            ProductionMiddlewareStage::Session,
            ProductionMiddlewareStage::Authentication,
            ProductionMiddlewareStage::Tenant,
            ProductionMiddlewareStage::Authorization,
            ProductionMiddlewareStage::RateLimit,
        ];
        let observed_identity_order = order
            .iter()
            .copied()
            .filter(|stage| expected_identity_order.contains(stage))
            .collect::<Vec<_>>();

        assert_eq!(observed_identity_order, expected_identity_order);
    }

    #[test]
    fn academy_preset_defaults_to_not_evaluated_and_fails_closed() -> Result<(), AcademyPresetError>
    {
        let preset = ProductionPreset::academy();
        let assessments = preset.assess([])?;

        assert_eq!(
            preset.middleware_order(),
            ProductionPreset::middleware_order()
        );
        assert_eq!(assessments.len(), AcademyProductionRequirement::ALL.len());
        assert!(
            assessments
                .iter()
                .all(|assessment| assessment.status() == AcademyCheckStatus::NotEvaluated)
        );
        assert!(matches!(
            preset.validate([]),
            Err(AcademyPresetError::RequirementNotPassed {
                status: AcademyCheckStatus::NotEvaluated,
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn academy_preset_requires_one_explicit_pass_per_requirement() {
        let preset = ProductionPreset::academy();
        let passes = AcademyProductionRequirement::ALL
            .iter()
            .copied()
            .map(|requirement| (requirement, AcademyCheckStatus::Pass));
        assert_eq!(preset.validate(passes), Ok(()));

        assert!(matches!(
            preset.validate([
                (
                    AcademyProductionRequirement::AuthenticatedIdentity,
                    AcademyCheckStatus::Pass,
                ),
                (
                    AcademyProductionRequirement::AuthenticatedIdentity,
                    AcademyCheckStatus::Fail,
                ),
            ]),
            Err(AcademyPresetError::DuplicateRequirement(
                AcademyProductionRequirement::AuthenticatedIdentity
            ))
        ));
        assert_eq!(AcademyCheckStatus::Skipped.as_str(), "SKIPPED");
    }
}
