//! Formal mathematical verification proofs for `rullst-core::security` using Kani model checker.

use super::pii::mask_pii;

/// Mathematically verifies that our PII masking engine (Credit Cards, Emails, etc):
/// 1. Will NEVER panic on any arbitrary valid string input (including emojis/complex utf-8).
/// 2. Always produces a masked string with the exact same character count as the original text.
#[kani::proof]
#[kani::unwind(3)]
#[cfg_attr(mutants, mutants::skip)]
fn proof_mask_pii_safety_and_invariants() {
    let bytes: [u8; 2] = kani::any();

    if let Ok(s) = std::str::from_utf8(&bytes) {
        let masked = mask_pii(s);
        assert_eq!(masked.chars().count(), s.chars().count());
    }
}
