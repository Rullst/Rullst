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
    /// Small-order and non-canonical point encodings are rejected: Ed25519
    /// verification does not check the key's order, so such a key would accept
    /// signatures forged without any private key.
    pub fn with_key(
        mut self,
        key_id: impl Into<String>,
        public_key: [u8; 32],
    ) -> Result<Self, AgeError> {
        let key_id = key_id.into();
        if !valid_token(&key_id)
            || degenerate_key(&public_key)
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

/// Canonical y coordinates of the eight small-order Edwards25519 points
/// (y = 0, 1, p - 1 and the two order-8 values); the x sign bit is ignored,
/// as in libsodium's `ge25519_has_small_order`.
const SMALL_ORDER_Y: [[u8; 32]; 5] = [
    [0; 32],
    [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x05,
    ],
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0x7a,
    ],
    p_minus_one(),
];

/// Little-endian p - 1 = 2^255 - 20.
const fn p_minus_one() -> [u8; 32] {
    let mut bytes = [0xff; 32];
    bytes[0] = 0xec;
    bytes[31] = 0x7f;
    bytes
}

/// True for a small-order point or a y coordinate that is not reduced mod p
/// (which includes the non-canonical encodings of y = 0 and y = 1).
fn degenerate_key(key: &[u8; 32]) -> bool {
    let mut y = *key;
    y[31] &= 0x7f;
    // y >= p = 2^255 - 19 exactly when bytes 1..=30 are 0xff, the top byte
    // (sign bit cleared) is 0x7f and the lowest byte is at least 0xed.
    let non_canonical = y[31] == 0x7f && y[1..31].iter().all(|byte| *byte == 0xff) && y[0] >= 0xed;
    non_canonical || SMALL_ORDER_Y.contains(&y)
}
