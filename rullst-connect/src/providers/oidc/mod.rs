pub mod discovery;
pub mod token;

#[cfg(test)]
mod tests;

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use subtle::ConstantTimeEq;

    #[kani::proof]
    #[kani::unwind(33)]
    fn verify_constant_time_eq_safety() {
        let len: usize = kani::any();
        kani::assume(len <= 32);

        let a: [u8; 32] = kani::any();
        let b: [u8; 32] = kani::any();

        let a_slice = &a[..len];
        let b_slice = &b[..len];
        let _ = a_slice.ct_eq(b_slice);
    }
}

pub use discovery::OidcProvider;
