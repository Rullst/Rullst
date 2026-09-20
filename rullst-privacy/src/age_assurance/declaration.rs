//! First-party declarations remain declarations, never verified age attributes.

use super::{
    AgeAssessment, AgeChallenge, AgeClock, AgeError, AgeMethod, AgeOutcome, AgePolicy,
    ReplayDurability, ReplayStore, SubjectBinding, SystemAgeClock, verifier::finish_assessment,
};
use serde::{Deserialize, Serialize};

/// An explicit answer to the exact threshold in the retained server challenge.
/// Do not infer an affirmative answer from a missing checkbox or request field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgeDeclaration {
    MeetsThreshold,
    BelowThreshold,
    Declined,
}

/// Provider-independent first-party declaration under a server-owned policy.
///
/// Authentication, CSRF, trusted challenge retention and explicit user choice
/// belong to the host. This gate validates the scope and consumes its nonce;
/// it does not determine whether the person's declaration is truthful. Stronger
/// methods and policies must use their own evaluated evidence path.
pub struct DeclarationGate<S> {
    store: S,
    development: bool,
}

impl<S: ReplayStore> DeclarationGate<S> {
    /// Production requires shared durable replay state, just like signed evidence.
    pub fn new(store: S) -> Result<Self, AgeError> {
        if store.durability() != ReplayDurability::SharedDurable {
            return Err(AgeError::DurableReplayRequired);
        }
        Ok(Self {
            store,
            development: false,
        })
    }

    /// Explicit development use with process-local state. The declared assurance
    /// still describes the answer, not a certification of its accuracy.
    pub fn for_development(store: S) -> Self {
        Self {
            store,
            development: true,
        }
    }

    pub async fn assess(
        &self,
        policy: &AgePolicy,
        binding: &SubjectBinding,
        challenge: &AgeChallenge,
        answer: AgeDeclaration,
    ) -> Result<AgeAssessment, AgeError> {
        self.assess_with_clock(policy, binding, challenge, answer, &SystemAgeClock)
            .await
    }

    /// Explicit trusted clock; client-provided timestamps are never appropriate.
    pub async fn assess_with_clock(
        &self,
        policy: &AgePolicy,
        binding: &SubjectBinding,
        challenge: &AgeChallenge,
        answer: AgeDeclaration,
        clock: &impl AgeClock,
    ) -> Result<AgeAssessment, AgeError> {
        let now = clock.now()?;
        challenge.validate(policy, binding, now)?;
        if challenge.method() != AgeMethod::SelfDeclaration {
            return Err(AgeError::MethodNotAllowed);
        }
        let outcome = match answer {
            AgeDeclaration::MeetsThreshold => AgeOutcome::MeetsThreshold,
            AgeDeclaration::BelowThreshold => AgeOutcome::BelowThreshold,
            AgeDeclaration::Declined => AgeOutcome::Inconclusive,
        };
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
}
