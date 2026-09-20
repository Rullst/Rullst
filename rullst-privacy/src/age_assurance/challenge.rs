use super::{AgeError, AgeMethod, AgePolicy, valid_token};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Opaque, preferably pairwise references resolved by the server, never email,
/// raw session cookies, document numbers or values trusted from form fields.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectBinding {
    subject: String,
    tenant: String,
    session: String,
    audience: String,
    action: String,
}

impl fmt::Debug for SubjectBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SubjectBinding([redacted])")
    }
}

impl SubjectBinding {
    pub fn new(
        subject: impl Into<String>,
        tenant: impl Into<String>,
        session: impl Into<String>,
        audience: impl Into<String>,
        action: impl Into<String>,
    ) -> Result<Self, AgeError> {
        let binding = Self {
            subject: subject.into(),
            tenant: tenant.into(),
            session: session.into(),
            audience: audience.into(),
            action: action.into(),
        };
        binding.validate()?;
        Ok(binding)
    }

    fn validate(&self) -> Result<(), AgeError> {
        if [
            &self.subject,
            &self.tenant,
            &self.session,
            &self.audience,
            &self.action,
        ]
        .iter()
        .any(|value| !valid_token(value))
        {
            return Err(AgeError::InvalidConfiguration);
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChallengeWire {
    pub binding: SubjectBinding,
    pub policy: AgePolicy,
    pub method: AgeMethod,
    pub threshold: u8,
    pub nonce: [u8; 32],
    pub issued_at: i64,
    pub expires_at: i64,
}

/// Retain this challenge on the server. JSON returned from a client must never
/// substitute for the server's challenge or authenticated binding.
#[derive(Clone)]
pub struct AgeChallenge(pub(crate) ChallengeWire);

impl fmt::Debug for AgeChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AgeChallenge([redacted])")
    }
}

impl AgeChallenge {
    /// Issues a random one-use challenge using a trusted server clock.
    pub fn issue(
        policy: &AgePolicy,
        binding: SubjectBinding,
        method: AgeMethod,
        now: i64,
    ) -> Result<Self, AgeError> {
        binding.validate()?;
        let threshold = policy.threshold(method)?;
        let expires_at = now
            .checked_add(policy.lifetime())
            .filter(|_| now >= 0)
            .ok_or(AgeError::InvalidChallenge)?;
        let mut nonce = [0; 32];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| AgeError::EntropyUnavailable)?;
        Ok(Self(ChallengeWire {
            binding,
            policy: policy.clone(),
            method,
            threshold,
            nonce,
            issued_at: now,
            expires_at,
        }))
    }

    /// The provider must determine this predicate. For facial estimation it
    /// includes the configured challenge margin, not just the minimum age.
    pub fn threshold(&self) -> u8 {
        self.0.threshold
    }

    pub fn method(&self) -> AgeMethod {
        self.0.method
    }

    pub fn expires_at(&self) -> i64 {
        self.0.expires_at
    }

    /// Serialize only for the trusted issuer integration. It contains opaque
    /// personal references and must not be sent to analytics or ordinary logs.
    pub fn request_json(&self) -> Result<Vec<u8>, AgeError> {
        serde_json::to_vec(&self.0).map_err(|_| AgeError::InvalidChallenge)
    }

    pub(crate) fn validate(
        &self,
        policy: &AgePolicy,
        binding: &SubjectBinding,
        now: i64,
    ) -> Result<(), AgeError> {
        if policy != &self.0.policy || binding != &self.0.binding {
            return Err(AgeError::BindingMismatch);
        }
        binding.validate()?;
        if policy.threshold(self.0.method)? != self.0.threshold || now < self.0.issued_at {
            return Err(AgeError::InvalidChallenge);
        }
        if self.0.issued_at < 0
            || self.0.issued_at.checked_add(policy.lifetime()) != Some(self.0.expires_at)
        {
            return Err(AgeError::InvalidChallenge);
        }
        if now >= self.0.expires_at {
            return Err(AgeError::Expired);
        }
        Ok(())
    }
}
