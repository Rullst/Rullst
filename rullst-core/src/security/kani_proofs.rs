//! Formal mathematical verification proofs for `rullst-core::security` using Kani model checker.

use super::pii::card_mask_count;

/// Proves the production card-mask boundary and arithmetic for every possible
/// `usize`: accepted card lengths are exactly 13 through 19 digits, the
/// subtraction cannot underflow, and exactly four digits remain unmasked.
#[kani::proof]
#[cfg_attr(mutants, mutants::skip)]
fn proof_card_mask_count_boundaries() {
    let digit_count: usize = kani::any();

    match card_mask_count(digit_count) {
        Some(mask_count) => {
            assert!(digit_count >= 13);
            assert!(digit_count <= 19);
            assert!(mask_count >= 9);
            assert!(mask_count <= 15);
            assert!(mask_count + 4 == digit_count);
        }
        None => {
            assert!(digit_count < 13 || digit_count > 19);
        }
    }
}
