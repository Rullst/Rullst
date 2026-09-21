//! Deterministic fuzz-only fixtures. No keys or nonces here are production defaults.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::hmac;
use rullst_privacy::age_assurance::*;
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

pub struct FixedClock(pub i64);
impl AgeClock for FixedClock {
    fn now(&self) -> Result<i64, AgeError> {
        Ok(self.0)
    }
}

pub fn policy() -> AgePolicy {
    AgePolicy::new("fuzz-policy-v1", RiskLevel::Low, 18).unwrap()
}

pub fn binding() -> SubjectBinding {
    SubjectBinding::new("subject", "tenant", "session", "audience", "action").unwrap()
}

pub fn tokens() -> ChallengeTokens {
    ChallengeTokens::new("fuzz-key", &[7; 32]).unwrap()
}

/// Authenticate arbitrary bytes to reach the parser beyond the authentication
/// boundary. The key is public and confined to this unpublished fuzz workspace.
pub fn authenticate_payload(payload: &[u8]) -> String {
    let message = format!("ra1.fuzz-key.{}", URL_SAFE_NO_PAD.encode(payload));
    let mut authenticated = b"rullst.age-challenge-token.v1\0".to_vec();
    authenticated.extend_from_slice(message.as_bytes());
    let tag = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, &[7; 32]), &authenticated);
    format!("{message}.{}", URL_SAFE_NO_PAD.encode(tag.as_ref()))
}

/// Reconstruct a deterministic fixture through the real authenticated transport;
/// avoid adding a production constructor that accepts caller-selected nonces.
pub fn challenge(method: AgeMethod) -> AgeChallenge {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "binding": binding(), "policy": policy(), "method": method,
        "threshold": if method == AgeMethod::FacialEstimation {21} else {18},
        "nonce": ([9_u8; 32]), "issued_at": 1000, "expires_at": 1300
    }))
    .unwrap();
    tokens()
        .open_with_clock(
            &authenticate_payload(&bytes),
            &policy(),
            &binding(),
            &FixedClock(1001),
        )
        .unwrap()
}

/// Preserve useful authenticated structure while allowing arbitrary inserted
/// bytes, removals and replacements; the whole input remains bounded by callers.
pub fn splice(original: &[u8], data: &[u8]) -> Vec<u8> {
    let offset = usize::from(u16::from_le_bytes([
        data.first().copied().unwrap_or(0),
        data.get(1).copied().unwrap_or(0),
    ])) % (original.len() + 1);
    let length = usize::from(u16::from_le_bytes([
        data.get(2).copied().unwrap_or(0),
        data.get(3).copied().unwrap_or(0),
    ]));
    let end = (offset + length).min(original.len());
    let mut bytes = original[..offset].to_vec();
    bytes.extend_from_slice(data.get(4..).unwrap_or_default());
    bytes.extend_from_slice(&original[end..]);
    bytes
}

/// MemoryReplayStore has no I/O or yielding operations. A newly pending future
/// is a harness incompatibility to review, never an ignored assessment.
pub fn ready<T>(future: impl Future<Output = T>) -> T {
    match Box::pin(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("fuzz memory-store future unexpectedly yielded"),
    }
}
