use super::{
    AgeChallenge, AgeClock, AgeError, AgeMethod, AgeOutcome, AgePolicy, ReplayDurability,
    ReplayStore, SubjectBinding, SystemAgeClock, TrustedIssuer,
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
    pub async fn verify(
        &self,
        policy: &AgePolicy,
        binding: &SubjectBinding,
        challenge: &AgeChallenge,
        payload: &[u8],
        signature: &[u8],
    ) -> Result<AgeAssessment, AgeError> {
        self.verify_with_clock(
            policy,
            binding,
            challenge,
            payload,
            signature,
            &SystemAgeClock,
        )
        .await
    }

    /// Explicit trusted-clock integration. Samples before validation and after
    /// asynchronous storage; expiry or rollback while waiting grants no access.
    pub async fn verify_with_clock(
        &self,
        policy: &AgePolicy,
        binding: &SubjectBinding,
        challenge: &AgeChallenge,
        payload: &[u8],
        signature: &[u8],
        clock: &impl AgeClock,
    ) -> Result<AgeAssessment, AgeError> {
        let now = clock.now()?;
        challenge.validate(policy, binding, now)?;
        let outcome = self.issuer.verify(challenge, payload, signature)?;
        finish_assessment(
            &self.store,
            self.development,
            challenge,
            outcome,
            false,
            now,
            clock,
        )
        .await
    }

    pub async fn verify_mock(
        &self,
        policy: &AgePolicy,
        binding: &SubjectBinding,
        challenge: &AgeChallenge,
        provider: &MockAgeProvider,
    ) -> Result<AgeAssessment, AgeError> {
        self.verify_mock_with_clock(policy, binding, challenge, provider, &SystemAgeClock)
            .await
    }

    pub async fn verify_mock_with_clock(
        &self,
        policy: &AgePolicy,
        binding: &SubjectBinding,
        challenge: &AgeChallenge,
        provider: &MockAgeProvider,
        clock: &impl AgeClock,
    ) -> Result<AgeAssessment, AgeError> {
        if !self.development {
            return Err(AgeError::MockInProduction);
        }
        let now = clock.now()?;
        challenge.validate(policy, binding, now)?;
        finish_assessment(
            &self.store,
            self.development,
            challenge,
            provider.outcome,
            true,
            now,
            clock,
        )
        .await
    }
}

pub(super) async fn finish_assessment<S: ReplayStore>(
    store: &S,
    development: bool,
    challenge: &AgeChallenge,
    outcome: AgeOutcome,
    mock: bool,
    now: i64,
    clock: &impl AgeClock,
) -> Result<AgeAssessment, AgeError> {
    // Recheck because an application adapter may change availability/mode.
    if !development && store.durability() != ReplayDurability::SharedDurable {
        return Err(AgeError::DurableReplayRequired);
    }
    if !store
        .claim(challenge.0.nonce, challenge.expires_at(), now)
        .await?
    {
        return Err(AgeError::Replay);
    }
    // The nonce stays consumed even when time or the final check fails.
    let completed_at = clock.now()?;
    if completed_at < now {
        return Err(AgeError::ClockRollback);
    }
    if completed_at >= challenge.expires_at() {
        return Err(AgeError::Expired);
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
