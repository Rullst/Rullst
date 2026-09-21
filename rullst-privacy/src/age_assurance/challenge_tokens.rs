//! Authenticated, bounded transport of server-issued challenges between hosts.

use super::{
    AgeChallenge, AgeClock, AgeError, AgePolicy, SubjectBinding, SystemAgeClock,
    challenge::ChallengeWire,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::hmac;
use std::{collections::BTreeMap, fmt};

const DOMAIN: &[u8] = b"rullst.age-challenge-token.v1\0";
const MAX_CHALLENGE_BYTES: usize = 4096;
const MAX_PAYLOAD_BASE64: usize = MAX_CHALLENGE_BYTES * 4 / 3 + 2;

/// HMAC-authenticated server challenges with explicit bounded key rotation.
///
/// Provision independent high-entropy 32-byte secrets, shared only by trusted
/// application hosts. Tokens authenticate opaque references; they do not encrypt
/// them or establish a user's identity or age. Opening a token grants no access.
/// Always assess it against the current authenticated context and consume it
/// through the appropriate gate/verifier and durable replay store.
#[derive(Clone)]
pub struct ChallengeTokens {
    active: String,
    keys: BTreeMap<String, hmac::Key>,
}

impl fmt::Debug for ChallengeTokens {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ChallengeTokens([redacted])")
    }
}

impl ChallengeTokens {
    /// Hard upper bound for the whole UTF-8 token before parsing or allocation.
    pub const MAX_TOKEN_BYTES: usize = 8192;

    /// Creates an active signing/verification key. IDs are 1..=64 ASCII letters,
    /// digits, underscores or hyphens. Empty/default all-zero secrets are rejected.
    pub fn new(key_id: impl Into<String>, secret: &[u8]) -> Result<Self, AgeError> {
        let key_id = key_id.into();
        let mut tokens = Self {
            active: key_id.clone(),
            keys: BTreeMap::new(),
        };
        tokens.insert_key(key_id, secret)?;
        Ok(tokens)
    }

    /// Adds a previous verification key without changing the active signing key.
    /// Retire keys by building the next configuration without them. Duplicate IDs
    /// and more than eight total keys fail; there is no online key discovery.
    pub fn with_previous_key(
        mut self,
        key_id: impl Into<String>,
        secret: &[u8],
    ) -> Result<Self, AgeError> {
        self.insert_key(key_id.into(), secret)?;
        Ok(self)
    }

    fn insert_key(&mut self, id: String, secret: &[u8]) -> Result<(), AgeError> {
        if !valid_key_id(&id)
            || secret.len() != 32
            || secret.iter().all(|byte| *byte == 0)
            || self.keys.len() >= 8
            || self.keys.contains_key(&id)
        {
            return Err(AgeError::InvalidConfiguration);
        }
        self.keys
            .insert(id, hmac::Key::new(hmac::HMAC_SHA256, secret));
        Ok(())
    }

    /// Authenticates a server-issued challenge for transport. Only challenge
    /// creation selects policy/context; never accept either from a browser.
    pub fn seal(&self, challenge: &AgeChallenge) -> Result<String, AgeError> {
        let bytes = challenge.request_json()?;
        if bytes.len() > MAX_CHALLENGE_BYTES {
            return Err(AgeError::InvalidChallengeToken);
        }
        let key = self
            .keys
            .get(&self.active)
            .ok_or(AgeError::InvalidConfiguration)?;
        let message = format!("ra1.{}.{}", self.active, URL_SAFE_NO_PAD.encode(bytes));
        let tag = hmac::sign(key, &authenticated_message(&message));
        Ok(format!(
            "{message}.{}",
            URL_SAFE_NO_PAD.encode(tag.as_ref())
        ))
    }

    /// Restores an authenticated server challenge under the current policy,
    /// authenticated context and server time. It does not consume or authorize it.
    pub fn open(
        &self,
        token: &str,
        policy: &AgePolicy,
        binding: &SubjectBinding,
    ) -> Result<AgeChallenge, AgeError> {
        self.open_with_clock(token, policy, binding, &SystemAgeClock)
    }

    /// Explicit trusted-clock variant. Neither the binding nor clock may come
    /// from untrusted request fields, even when a token is cryptographically valid.
    pub fn open_with_clock(
        &self,
        token: &str,
        policy: &AgePolicy,
        binding: &SubjectBinding,
        clock: &impl AgeClock,
    ) -> Result<AgeChallenge, AgeError> {
        if token.len() > Self::MAX_TOKEN_BYTES {
            return Err(AgeError::InvalidChallengeToken);
        }
        let mut fields = token.split('.');
        let version = fields.next().ok_or(AgeError::InvalidChallengeToken)?;
        let id = fields.next().ok_or(AgeError::InvalidChallengeToken)?;
        let payload = fields.next().ok_or(AgeError::InvalidChallengeToken)?;
        let tag = fields.next().ok_or(AgeError::InvalidChallengeToken)?;
        if fields.next().is_some()
            || version != "ra1"
            || !valid_key_id(id)
            || payload.is_empty()
            || payload.len() > MAX_PAYLOAD_BASE64
            || tag.len() != 43
        {
            return Err(AgeError::InvalidChallengeToken);
        }
        let key = self.keys.get(id).ok_or(AgeError::InvalidChallengeToken)?;
        let tag = URL_SAFE_NO_PAD
            .decode(tag)
            .map_err(|_| AgeError::InvalidChallengeToken)?;
        let (message, _) = token
            .rsplit_once('.')
            .ok_or(AgeError::InvalidChallengeToken)?;
        hmac::verify(key, &authenticated_message(message), &tag)
            .map_err(|_| AgeError::InvalidChallengeToken)?;
        // Authentication precedes interpretation of every payload field.
        let bytes = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| AgeError::InvalidChallengeToken)?;
        if bytes.len() > MAX_CHALLENGE_BYTES {
            return Err(AgeError::InvalidChallengeToken);
        }
        let wire: ChallengeWire =
            serde_json::from_slice(&bytes).map_err(|_| AgeError::InvalidChallengeToken)?;
        let challenge = AgeChallenge(wire);
        challenge.validate(policy, binding, clock.now()?)?;
        Ok(challenge)
    }
}

fn valid_key_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn authenticated_message(message: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(DOMAIN.len() + message.len());
    bytes.extend_from_slice(DOMAIN);
    bytes.extend_from_slice(message.as_bytes());
    bytes
}
