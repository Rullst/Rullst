//! Age evidence is scoped to a server-owned policy, action and authenticated
//! context. Signed attestations prove what a configured issuer asserted, not
//! that the issuer's age determination was accurate or legally sufficient.

mod attestation;
mod challenge;
mod error;
mod policy;
mod replay;
mod verifier;

pub use attestation::{AgeOutcome, TrustedIssuer, encode_attestation, signing_message};
pub use challenge::{AgeChallenge, SubjectBinding};
pub use error::AgeError;
pub use policy::{AgeMethod, AgePolicy, RiskLevel};
pub use replay::{MemoryReplayStore, ReplayDurability, ReplayStore};
pub use verifier::{AgeAssessment, AgeDecision, AgeVerifier, Assurance, MockAgeProvider};

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}
