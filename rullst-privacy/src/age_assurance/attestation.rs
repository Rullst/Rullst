use super::{AgeChallenge, AgeError, AgeMethod, challenge::ChallengeWire, valid_token};
use ring::signature;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_ATTESTATION_BYTES: usize = 4096;
const SIGNING_CONTEXT: &[u8] = b"rullst.age-assurance.v1\0";

/// Predicate result from the selected method. For a facial challenge,
/// `BelowThreshold` requires an alternative; it does not establish minority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgeOutcome {
    MeetsThreshold,
    BelowThreshold,
    Inconclusive,
    Unavailable,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attestation {
    version: u8,
    issuer: String,
    key_id: String,
    challenge: ChallengeWire,
    outcome: AgeOutcome,
}

/// Encode a predicate for an issuer or a reviewed provider bridge to sign.
/// Call only after determining the requested predicate with the exact method.
/// Merely signing client assertions does not verify anybody's age.
pub fn encode_attestation(
    issuer: impl Into<String>,
    key_id: impl Into<String>,
    challenge: &AgeChallenge,
    outcome: AgeOutcome,
) -> Result<Vec<u8>, AgeError> {
    let issuer = issuer.into();
    let key_id = key_id.into();
    if !valid_token(&issuer) || !valid_token(&key_id) {
        return Err(AgeError::InvalidConfiguration);
    }
    let bytes = serde_json::to_vec(&Attestation {
        version: 1,
        issuer,
        key_id,
        challenge: challenge.0.clone(),
        outcome,
    })
    .map_err(|_| AgeError::InvalidAttestation)?;
    if bytes.len() > MAX_ATTESTATION_BYTES {
        return Err(AgeError::InvalidAttestation);
    }
    Ok(bytes)
}

/// Ed25519 signs this domain-separated message containing the exact JSON bytes.
/// There is no algorithm negotiation, network key discovery or JWT inference.
pub fn signing_message(payload: &[u8]) -> Result<Vec<u8>, AgeError> {
    if payload.is_empty() || payload.len() > MAX_ATTESTATION_BYTES {
        return Err(AgeError::InvalidAttestation);
    }
    let mut message = Vec::with_capacity(SIGNING_CONTEXT.len() + payload.len());
    message.extend_from_slice(SIGNING_CONTEXT);
    message.extend_from_slice(payload);
    Ok(message)
}

/// An explicitly trusted issuer with at most eight pinned Ed25519 keys.
/// The host must evaluate the issuer's methods, accuracy and data processing.
#[derive(Clone)]
pub struct TrustedIssuer {
    name: String,
    keys: BTreeMap<String, [u8; 32]>,
    methods: Vec<AgeMethod>,
}

impl TrustedIssuer {
    pub fn new(
        name: impl Into<String>,
        key_id: impl Into<String>,
        public_key: [u8; 32],
        methods: impl IntoIterator<Item = AgeMethod>,
    ) -> Result<Self, AgeError> {
        let name = name.into();
        if !valid_token(&name) {
            return Err(AgeError::InvalidConfiguration);
        }
        let mut accepted = Vec::new();
        for method in methods {
            if accepted.contains(&method) || accepted.len() >= 3 {
                return Err(AgeError::InvalidConfiguration);
            }
            accepted.push(method);
        }
        if accepted.is_empty() {
            return Err(AgeError::InvalidConfiguration);
        }
        Self {
            name,
            keys: BTreeMap::new(),
            methods: accepted,
        }
        .with_key(key_id, public_key)
    }

    /// Add an overlapping rotation key. Duplicate IDs are rejected. Retire keys
    /// by constructing the next issuer configuration while retaining replay state.
    pub fn with_key(
        mut self,
        key_id: impl Into<String>,
        public_key: [u8; 32],
    ) -> Result<Self, AgeError> {
        let key_id = key_id.into();
        if !valid_token(&key_id)
            || public_key == [0; 32]
            || self.keys.contains_key(&key_id)
            || self.keys.len() >= 8
        {
            return Err(AgeError::InvalidConfiguration);
        }
        self.keys.insert(key_id, public_key);
        Ok(self)
    }

    pub(crate) fn verify(
        &self,
        expected: &AgeChallenge,
        payload: &[u8],
        signature_bytes: &[u8],
    ) -> Result<AgeOutcome, AgeError> {
        if !self.methods.contains(&expected.method()) {
            return Err(AgeError::MethodNotAllowed);
        }
        let message = signing_message(payload)?;
        if signature_bytes.len() != 64 {
            return Err(AgeError::InvalidSignature);
        }
        let attestation: Attestation =
            serde_json::from_slice(payload).map_err(|_| AgeError::InvalidAttestation)?;
        if attestation.version != 1 || attestation.issuer != self.name {
            return Err(AgeError::InvalidAttestation);
        }
        let key = self
            .keys
            .get(&attestation.key_id)
            .ok_or(AgeError::InvalidSignature)?;
        signature::UnparsedPublicKey::new(&signature::ED25519, key)
            .verify(&message, signature_bytes)
            .map_err(|_| AgeError::InvalidSignature)?;
        if attestation.challenge != expected.0 {
            return Err(AgeError::BindingMismatch);
        }
        Ok(attestation.outcome)
    }
}
