use crate::{ContentHash, LabError, WorkerOutput, authorization::checked_time};
use serde::{Deserialize, Serialize};

/// Statement by the trusted controller AFTER verified execution and teardown.
/// A signature authenticates that authority; it does not independently prove
/// sandbox safety. No signing secret may enter the application or worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReceipt {
    pub output: WorkerOutput,
    pub started_at: i64,
    pub finished_at: i64,
    /// Digest of actual per-job namespace/resource/filesystem/network probes.
    pub observation_digest: ContentHash,
    /// Distinguishes confirmed empty group/workspace cleanup from uncertain loss.
    pub teardown: Teardown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Teardown {
    Confirmed,
    Uncertain,
}
impl ExecutionReceipt {
    pub fn validate(&self) -> Result<(), LabError> {
        checked_time(self.started_at)?;
        checked_time(self.finished_at)?;
        if self.finished_at < self.started_at || self.finished_at - self.started_at > 90 {
            return Err(LabError::Protocol);
        }
        self.output.validate()
    }
    #[cfg(any(feature = "sqlite", feature = "receipt-signing"))]
    fn bytes(&self) -> Result<Vec<u8>, LabError> {
        self.validate()?;
        serde_json::to_vec(&("RullstLabsReceipt-v1", crate::PROTOCOL_VERSION, self))
            .map_err(|_| LabError::Protocol)
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedReceipt {
    pub receipt: ExecutionReceipt,
    signature: String,
}
impl std::fmt::Debug for SignedReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignedReceipt")
            .field("receipt", &self.receipt)
            .finish_non_exhaustive()
    }
}
impl SignedReceipt {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LabError> {
        if bytes.len() > 20_480 {
            return Err(LabError::Capacity);
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| LabError::Protocol)?;
        value.receipt.validate()?;
        if value.signature.len() != 128
            || !value
                .signature
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(LabError::Protocol);
        }
        Ok(value)
    }
    #[cfg(feature = "sqlite")]
    pub(crate) fn verify(&self, public_key: &ContentHash) -> Result<(), LabError> {
        let key = hex::decode(public_key.as_str()).map_err(|_| LabError::Integrity)?;
        if self.signature.len() != 128
            || !self
                .signature
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(LabError::Integrity);
        }
        let signature = hex::decode(&self.signature).map_err(|_| LabError::Integrity)?;
        ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key)
            .verify(&self.receipt.bytes()?, &signature)
            .map_err(|_| LabError::Integrity)
    }
}
#[cfg(feature = "receipt-signing")]
pub struct ReceiptSigner {
    key: ring::signature::Ed25519KeyPair,
}
#[cfg(feature = "receipt-signing")]
impl ReceiptSigner {
    /// Dedicated controller seed. Never reuse an application/session/content key.
    pub fn from_seed(seed: zeroize::Zeroizing<[u8; 32]>) -> Result<Self, LabError> {
        if seed.iter().all(|b| *b == 0) {
            return Err(LabError::Configuration);
        }
        Ok(Self {
            key: ring::signature::Ed25519KeyPair::from_seed_unchecked(seed.as_ref())
                .map_err(|_| LabError::Configuration)?,
        })
    }
    pub fn public_key(&self) -> Result<ContentHash, LabError> {
        use ring::signature::KeyPair;
        ContentHash::new(hex::encode(self.key.public_key().as_ref()))
    }
    pub fn sign(&self, receipt: ExecutionReceipt) -> Result<SignedReceipt, LabError> {
        let signature = hex::encode(self.key.sign(&receipt.bytes()?).as_ref());
        Ok(SignedReceipt { receipt, signature })
    }
}
#[cfg(feature = "receipt-signing")]
impl std::fmt::Debug for ReceiptSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReceiptSigner([redacted])")
    }
}
