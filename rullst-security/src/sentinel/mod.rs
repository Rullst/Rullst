//! Deterministic anomaly assessment and an opt-in one-shot proof-of-work gate.

mod classifier;
mod pow;

pub use classifier::{
    SentinelAction, SentinelAssessment, SentinelObservation, SentinelPolicy, ThreatClassifier,
    ThreatPattern,
};
pub use pow::{ProofOfWorkChallenge, ProofOfWorkConfig, ProofOfWorkGate};

/// Bounded Sentinel or challenge-protocol failure.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum SentinelError {
    #[error("invalid Sentinel configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("invalid Sentinel observation: {0}")]
    InvalidObservation(&'static str),
    #[error("Sentinel subject must be a trimmed 1-256 byte value without control characters")]
    InvalidSubject,
    #[error("Sentinel PoW key must contain at least 32 strong bytes")]
    WeakKey,
    #[error("Sentinel cryptographic state could not be initialized")]
    CryptoInitialization,
    #[error("operating-system randomness is unavailable")]
    RandomnessUnavailable,
    #[error("system time is unavailable")]
    ClockUnavailable,
    #[error("active proof-of-work challenge capacity reached")]
    CapacityReached,
    #[error("invalid proof-of-work token")]
    InvalidToken,
    #[error("proof-of-work challenge expired")]
    ExpiredChallenge,
    #[error("proof-of-work solution is invalid")]
    InvalidProof,
    #[error("proof-of-work challenge is unknown or already consumed")]
    ReplayOrUnknownChallenge,
}

/// Assessment plus an optional client-facing challenge.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct SentinelOutcome {
    assessment: SentinelAssessment,
    challenge: Option<ProofOfWorkChallenge>,
}

impl SentinelOutcome {
    pub fn assessment(&self) -> &SentinelAssessment {
        &self.assessment
    }

    pub fn challenge(&self) -> Option<&ProofOfWorkChallenge> {
        self.challenge.as_ref()
    }
}

/// One-call composition of transparent classification and one-shot PoW.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ThreatSentinel {
    classifier: ThreatClassifier,
    proof_of_work: ProofOfWorkGate,
}

impl ThreatSentinel {
    /// Creates an opt-in Sentinel with caller-selected transparent policies.
    pub fn try_new(
        key: impl AsRef<[u8]>,
        classifier_policy: SentinelPolicy,
        proof_policy: ProofOfWorkConfig,
    ) -> Result<Self, SentinelError> {
        Ok(Self {
            classifier: ThreatClassifier::new(classifier_policy),
            proof_of_work: ProofOfWorkGate::try_new(key, proof_policy)?,
        })
    }

    /// Assesses trusted aggregates and issues PoW only for a named detected pattern.
    pub fn assess(
        &self,
        subject: impl Into<String>,
        observation: SentinelObservation,
    ) -> Result<SentinelOutcome, SentinelError> {
        let assessment = self.classifier.assess(observation);
        let challenge = if assessment.action() == SentinelAction::ProofOfWork {
            Some(self.proof_of_work.issue(subject)?)
        } else {
            None
        };
        Ok(SentinelOutcome {
            assessment,
            challenge,
        })
    }

    /// Verifies and atomically consumes one issued challenge.
    pub fn verify(
        &self,
        subject: impl Into<String>,
        token: impl Into<String>,
        solution_nonce: u64,
    ) -> Result<(), SentinelError> {
        self.proof_of_work.verify(subject, token, solution_nonce)
    }

    pub fn classifier(&self) -> &ThreatClassifier {
        &self.classifier
    }

    pub fn proof_of_work(&self) -> &ProofOfWorkGate {
        &self.proof_of_work
    }
}

#[cfg(test)]
mod tests;
