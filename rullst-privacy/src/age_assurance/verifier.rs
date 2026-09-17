use super::{
    AgeChallenge, AgeError, AgeMethod, AgeOutcome, AgePolicy, ReplayDurability, ReplayStore,
    SubjectBinding, TrustedIssuer,
};

/// The assertion's method, not a certification of the provider or application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assurance {
    Declared,
    Estimated,
    VerifiedAttribute,
    OfflineMock,
}

/// Only `Allowed` permits the particular age-gated action. All other decisions
/// need denial or an appropriate alternative; they are not permission to bypass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeDecision {
    Allowed,
    BelowMinimumAge,
    AlternativeRequired,
    Unavailable,
}

/// Minimized result. It deliberately contains no photo, birth date or subject ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgeAssessment {
    decision: AgeDecision,
    assurance: Assurance,
    policy_version: String,
    expires_at: i64,
}

impl AgeAssessment {
    pub fn decision(&self) -> AgeDecision {
        self.decision
    }

    pub fn assurance(&self) -> Assurance {
        self.assurance
    }

    pub fn policy_version(&self) -> &str {
        &self.policy_version
    }

    /// Upper bound inherited from the one-use challenge; this object is not a
    /// reusable bearer credential. Hosts must not authorize another context with it.
    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }
}

/// Deterministic offline predicate fixture; it never examines a person's face.
pub struct MockAgeProvider {
    outcome: AgeOutcome,
}

impl MockAgeProvider {
    /// Empty or `mock_*` credentials select offline behavior. Other values fail
    /// explicitly; there is no implicit remote-provider implementation.
    pub fn new(credentials: impl Into<String>, outcome: AgeOutcome) -> Result<Self, AgeError> {
        let credentials = credentials.into();
        if !credentials.is_empty() && !credentials.starts_with("mock_") {
            return Err(AgeError::InvalidConfiguration);
        }
        Ok(Self { outcome })
    }
}

/// Static-dispatch verifier; authentication and policy selection happen in the
/// application before this boundary. Production is the default constructor.
pub struct AgeVerifier<S> {
    issuer: TrustedIssuer,
    store: S,
    development: bool,
}

impl<S: ReplayStore> AgeVerifier<S> {
    pub fn new(issuer: TrustedIssuer, store: S) -> Result<Self, AgeError> {
        if store.durability() != ReplayDurability::SharedDurable {
            return Err(AgeError::DurableReplayRequired);
        }
        Ok(Self {
            issuer,
            store,
            development: false,
        })
    }

    /// Explicit development boundary, permitting a process-local store and mocks.
    pub fn for_development(issuer: TrustedIssuer, store: S) -> Self {
        Self {
            issuer,
            store,
            development: true,
        }
    }

    /// Verify exact signed bytes against the retained challenge and current
    /// authenticated context. Uses server time; claims the nonce before returning
    /// any decision. The caller must enforce `Allowed` only for this action.
    pub fn verify(
        &self,
        policy: &AgePolicy,
        binding: &SubjectBinding,
        challenge: &AgeChallenge,
        payload: &[u8],
        signature: &[u8],
        now: i64,
    ) -> Result<AgeAssessment, AgeError> {
        challenge.validate(policy, binding, now)?;
        let outcome = self.issuer.verify(challenge, payload, signature)?;
        self.finish(challenge, outcome, false, now)
    }

    pub fn verify_mock(
        &self,
        policy: &AgePolicy,
        binding: &SubjectBinding,
        challenge: &AgeChallenge,
        provider: &MockAgeProvider,
        now: i64,
    ) -> Result<AgeAssessment, AgeError> {
        if !self.development {
            return Err(AgeError::MockInProduction);
        }
        challenge.validate(policy, binding, now)?;
        self.finish(challenge, provider.outcome, true, now)
    }

    fn finish(
        &self,
        challenge: &AgeChallenge,
        outcome: AgeOutcome,
        mock: bool,
        now: i64,
    ) -> Result<AgeAssessment, AgeError> {
        // Recheck because an application adapter may change availability/mode.
        if !self.development && self.store.durability() != ReplayDurability::SharedDurable {
            return Err(AgeError::DurableReplayRequired);
        }
        if !self
            .store
            .claim(challenge.0.nonce, challenge.expires_at(), now)?
        {
            return Err(AgeError::Replay);
        }
        let method = challenge.method();
        let decision = match outcome {
            AgeOutcome::MeetsThreshold => AgeDecision::Allowed,
            AgeOutcome::BelowThreshold if method == AgeMethod::FacialEstimation => {
                AgeDecision::AlternativeRequired
            }
            AgeOutcome::BelowThreshold => AgeDecision::BelowMinimumAge,
            AgeOutcome::Inconclusive => AgeDecision::AlternativeRequired,
            AgeOutcome::Unavailable => AgeDecision::Unavailable,
        };
        let assurance = if mock {
            Assurance::OfflineMock
        } else {
            match method {
                AgeMethod::SelfDeclaration => Assurance::Declared,
                AgeMethod::FacialEstimation => Assurance::Estimated,
                AgeMethod::VerifiedAttribute => Assurance::VerifiedAttribute,
            }
        };
        Ok(AgeAssessment {
            decision,
            assurance,
            policy_version: challenge.0.policy.version().to_owned(),
            expires_at: challenge.expires_at(),
        })
    }
}
