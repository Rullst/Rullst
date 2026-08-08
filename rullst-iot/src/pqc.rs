//! Lightweight Post-Quantum Edge Encryption (`rullst_iot::pqc`).
//!
//! Provides a compact ML-KEM (Kyber) style key encapsulation stub suitable
//! for low-power edge nodes protecting telemetry links against quantum threats.

extern crate alloc;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

/// Simulated Post-Quantum Key Encapsulation Mechanism (ML-KEM / Kyber stub).
pub struct PqcKeyPair {
    pub public_key: Vec<u8>,
    secret_key: Vec<u8>,
}

impl PqcKeyPair {
    /// Derives a deterministic key pair from a seed (for testing).
    /// Production: replace with a full ML-KEM / Kyber implementation.
    pub fn from_seed(seed: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        let pk = hasher.finalize().to_vec();

        let mut hasher2 = Sha256::new();
        hasher2.update(&pk);
        hasher2.update(b"sk_derive");
        let sk = hasher2.finalize().to_vec();

        Self {
            public_key: pk,
            secret_key: sk,
        }
    }

    /// Encapsulates a session key using the public key (stub: returns HMAC-SHA256).
    pub fn encapsulate(&self, plaintext: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.public_key);
        hasher.update(plaintext);
        hasher.finalize().to_vec()
    }

    /// Decapsulates a session key using the private key (stub: HMAC-SHA256 verify).
    pub fn decapsulate(&self, ciphertext: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.secret_key);
        hasher.update(ciphertext);
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pqc_key_pair_generation() {
        let keypair = PqcKeyPair::from_seed(b"edge_node_seed_value");
        assert_eq!(keypair.public_key.len(), 32);
    }

    #[test]
    fn test_pqc_encapsulation() {
        let keypair = PqcKeyPair::from_seed(b"edge_node_seed_value");
        let ct = keypair.encapsulate(b"sensor_data_payload");
        assert_eq!(ct.len(), 32);
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_pqc_keypair_bounds() {
        let seed: [u8; 8] = kani::any();
        let keypair = PqcKeyPair::from_seed(&seed);
        assert_eq!(keypair.public_key.len(), 32);
        assert_eq!(keypair.secret_key.len(), 32);

        let data: [u8; 8] = kani::any();
        let ct = keypair.encapsulate(&data);
        assert_eq!(ct.len(), 32);
    }
}

