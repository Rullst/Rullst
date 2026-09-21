//! Age evidence is scoped to a server-owned policy, action and authenticated
//! context. Signed attestations prove what a configured issuer asserted, not
//! that the issuer's age determination was accurate or legally sufficient.

mod attestation;
mod challenge;
#[cfg(feature = "challenge-tokens")]
mod challenge_tokens;
mod clock;
mod declaration;
mod error;
mod policy;
#[cfg(feature = "postgres")]
mod postgres;
mod replay;
#[cfg(feature = "sqlite")]
mod sqlite;
mod verifier;

pub use attestation::{AgeOutcome, TrustedIssuer, encode_attestation, signing_message};
pub use challenge::{AgeChallenge, SubjectBinding};
#[cfg(feature = "challenge-tokens")]
pub use challenge_tokens::ChallengeTokens;
pub use clock::{AgeClock, SystemAgeClock};
pub use declaration::{AgeDeclaration, DeclarationGate};
pub use error::AgeError;
pub use policy::{AgeMethod, AgePolicy, RiskLevel};
#[cfg(feature = "postgres")]
pub use postgres::PostgresReplayStore;
pub use replay::{MemoryReplayStore, ReplayDurability, ReplayStore};
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteReplayStore;
pub use verifier::{AgeAssessment, AgeDecision, AgeVerifier, Assurance, MockAgeProvider};

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}
