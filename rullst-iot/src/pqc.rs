//! Deterministic post-quantum-shaped fixtures for tests and demos.
//!
//! This module is available only with `experimental-simulators`. It does not
//! implement ML-KEM, Kyber, a KEM round trip, confidentiality, or quantum safety.

use sha2::{Digest, Sha256};

/// Deterministic fixture data; not a post-quantum key pair.
#[non_exhaustive]
pub struct SimulatedPqcFixture {
    public_fixture: [u8; 32],
    private_fixture: [u8; 32],
}

impl SimulatedPqcFixture {
    /// Derives reproducible, non-secret fixture bytes from a test seed.
    #[must_use]
    pub fn from_seed(seed: &[u8]) -> Self {
        let mut public_hasher = Sha256::new();
        public_hasher.update(b"RULLST-SIMULATED-PQC-PUBLIC-V1\0");
        public_hasher.update(seed);
        let public_fixture = public_hasher.finalize().into();

        let mut private_hasher = Sha256::new();
        private_hasher.update(b"RULLST-SIMULATED-PQC-PRIVATE-V1\0");
        private_hasher.update(seed);
        let private_fixture = private_hasher.finalize().into();

        Self {
            public_fixture,
            private_fixture,
        }
    }

    /// Returns public fixture bytes with no cryptographic guarantees.
    #[must_use]
    pub fn public_fixture(&self) -> [u8; 32] {
        self.public_fixture
    }

    /// Produces a deterministic ciphertext-shaped fixture, not KEM encapsulation.
    #[must_use]
    pub fn derive_ciphertext_fixture(&self, input: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"RULLST-SIMULATED-PQC-CIPHERTEXT-V1\0");
        hasher.update(self.public_fixture);
        hasher.update(input);
        hasher.finalize().into()
    }

    /// Produces deterministic output-shaped bytes, not KEM decapsulation.
    #[must_use]
    pub fn derive_output_fixture(&self, input: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"RULLST-SIMULATED-PQC-OUTPUT-V1\0");
        hasher.update(self.private_fixture);
        hasher.update(input);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulated_pqc_fixtures_are_deterministic_and_not_a_kem_claim() {
        let fixture = SimulatedPqcFixture::from_seed(b"documented test seed");
        assert_eq!(fixture.public_fixture(), fixture.public_fixture());
        assert_ne!(
            fixture.derive_ciphertext_fixture(b"a"),
            fixture.derive_ciphertext_fixture(b"b")
        );
    }
}
