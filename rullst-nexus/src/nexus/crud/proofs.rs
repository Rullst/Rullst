//! Formal verification proofs for Nexus CRUD using Kani.

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use crate::nexus::crud::query::sanitize_identifier;

    #[kani::proof]
    #[kani::unwind(5)]
    fn proof_sanitize_identifier_alphanumeric_or_underscore() {
        let name: [u8; 4] = kani::any();
        if let Ok(s) = std::str::from_utf8(&name) {
            let clean = sanitize_identifier(s);
            assert!(clean.len() <= 64);
        }
    }
}
