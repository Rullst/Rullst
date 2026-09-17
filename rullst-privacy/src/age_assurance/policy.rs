use super::{AgeError, valid_token};
use serde::{Deserialize, Serialize};

/// Engineering presets, not jurisdiction-specific legal classifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// A clearly labeled declaration may suffice for the application's purpose.
    Low,
    /// Require a reviewed facial estimate or verified age attribute.
    Elevated,
    /// Require a verified age attribute; declaration/estimation alone is denied.
    Restricted,
}

/// Distinct methods must never be silently promoted to a stronger assurance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgeMethod {
    SelfDeclaration,
    FacialEstimation,
    VerifiedAttribute,
}

/// Immutable server-owned policy. Use a new version when legal/risk decisions
/// change. This type has no built-in country rules or universal minimum age.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgePolicy {
    version: String,
    risk: RiskLevel,
    minimum_age: u8,
    estimation_margin: u8,
    validity_seconds: u16,
}

impl AgePolicy {
    /// Creates a policy with a five-minute challenge and a three-year facial
    /// challenge margin. That margin is a configurable engineering default,
    /// not evaluated accuracy evidence for any model or audience.
    pub fn new(
        version: impl Into<String>,
        risk: RiskLevel,
        minimum_age: u8,
    ) -> Result<Self, AgeError> {
        let policy = Self {
            version: version.into(),
            risk,
            minimum_age,
            estimation_margin: 3,
            validity_seconds: 300,
        };
        policy.validate()?;
        Ok(policy)
    }

    /// Configure the positive margin from the selected estimator's evaluation.
    pub fn with_estimation_margin(mut self, years: u8) -> Result<Self, AgeError> {
        self.estimation_margin = years;
        self.validate()?;
        Ok(self)
    }

    /// Sets challenge lifetime, bounded to 1..=900 seconds.
    pub fn with_validity_seconds(mut self, seconds: u16) -> Result<Self, AgeError> {
        self.validity_seconds = seconds;
        self.validate()?;
        Ok(self)
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn minimum_age(&self) -> u8 {
        self.minimum_age
    }

    /// Methods permitted by the selected risk preset.
    pub fn permits(&self, method: AgeMethod) -> bool {
        match self.risk {
            RiskLevel::Low => true,
            RiskLevel::Elevated => method != AgeMethod::SelfDeclaration,
            RiskLevel::Restricted => method == AgeMethod::VerifiedAttribute,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), AgeError> {
        if !valid_token(&self.version)
            || !(1..=120).contains(&self.minimum_age)
            || !(1..=20).contains(&self.estimation_margin)
            || !(1..=900).contains(&self.validity_seconds)
        {
            return Err(AgeError::InvalidConfiguration);
        }
        Ok(())
    }

    pub(crate) fn lifetime(&self) -> i64 {
        i64::from(self.validity_seconds)
    }

    pub(crate) fn threshold(&self, method: AgeMethod) -> Result<u8, AgeError> {
        self.validate()?;
        if !self.permits(method) {
            return Err(AgeError::MethodNotAllowed);
        }
        match method {
            AgeMethod::FacialEstimation => self
                .minimum_age
                .checked_add(self.estimation_margin)
                .filter(|threshold| *threshold <= 120)
                .ok_or(AgeError::InvalidConfiguration),
            _ => Ok(self.minimum_age),
        }
    }
}
