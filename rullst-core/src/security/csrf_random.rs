//! Random values of the double-submit CSRF protection.
//!
//! The cookie token is the secret: its characters come straight from the
//! operating system's random source. The per-response mask only has to make
//! every embedded value different (BREACH resistance, see `csrf_mask`), so it
//! comes from a ChaCha12 generator per thread, seeded from the operating
//! system and reseeded after every [`RESEED_AFTER_BYTES`] bytes, instead of a
//! system call per response. Either source failing yields `None`, which the
//! middleware answers with `500`: a page never embeds an unmasked token.

use rand::rngs::{StdRng, SysRng};
use rand::{Rng, SeedableRng, TryRng};
use std::cell::RefCell;

/// Length of a minted cookie token.
const TOKEN_LENGTH: usize = 32;

/// The characters of a minted token: `generate_csrf_token`'s alphabet.
const ALPHANUMERIC: &[u8; 62] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Operating-system bytes drawn at a time for a token. A byte's top six bits
/// pick a character, or none for the two values past the alphabet; 40 bytes
/// give 32 characters except about three times in a million, which draw
/// again.
const TOKEN_DRAW: usize = 40;

/// Draws a token may take before minting fails, far beyond any real need.
const TOKEN_DRAWS: usize = 4;

/// Mask bytes a thread's generator produces before it is reseeded from the
/// operating system: 64 KiB, as `rand`'s `ThreadRng`, about 2,000 masks of a
/// minted token.
pub(crate) const RESEED_AFTER_BYTES: usize = 64 * 1024;

/// A thread's mask generator and the bytes it may still produce.
pub(crate) struct MaskGenerator {
    rng: StdRng,
    remaining: usize,
}

thread_local! {
    static MASK_GENERATOR: RefCell<Option<MaskGenerator>> = const { RefCell::new(None) };
}

/// Fills `bytes` from the operating system's random source; `None` when it
/// fails.
fn os_fill(bytes: &mut [u8]) -> Option<()> {
    #[cfg(test)]
    if failure::os_source_fails() {
        return None;
    }
    SysRng.try_fill_bytes(bytes).ok()
}

/// A ChaCha12 generator seeded from the operating system's random source, or
/// `None` when it fails.
fn os_seeded() -> Option<StdRng> {
    let mut seed = [0_u8; 32];
    os_fill(&mut seed)?;
    Some(StdRng::from_seed(seed))
}

/// A new `rullst_csrf` cookie token: 32 ASCII letters and digits, each one
/// six bits of the operating system's random source (uniform, since bytes
/// past the alphabet are skipped). `None` when that source fails.
pub(crate) fn mint_cookie_token() -> Option<String> {
    let mut token = String::with_capacity(TOKEN_LENGTH);
    let mut bytes = [0_u8; TOKEN_DRAW];
    for _ in 0..TOKEN_DRAWS {
        os_fill(&mut bytes)?;
        for byte in bytes {
            if token.len() < TOKEN_LENGTH
                && let Some(&symbol) = ALPHANUMERIC.get(usize::from(byte >> 2))
            {
                token.push(char::from(symbol));
            }
        }
        if token.len() == TOKEN_LENGTH {
            return Some(token);
        }
    }
    None
}

/// Fills `mask` from this thread's generator. `None` when the operating
/// system's random source fails to seed or reseed it.
pub(crate) fn fill_mask(mask: &mut [u8]) -> Option<()> {
    MASK_GENERATOR
        .try_with(|state| fill_from(&mut *state.try_borrow_mut().ok()?, os_seeded, mask))
        .ok()
        .flatten()
}

/// Fills `mask` from `state`'s generator, first replacing it with one from
/// `seed` when there is none or it cannot produce `mask.len()` more bytes
/// before its reseed. `None`, with `state` unchanged, when `seed` fails.
pub(crate) fn fill_from(
    state: &mut Option<MaskGenerator>,
    seed: impl FnOnce() -> Option<StdRng>,
    mask: &mut [u8],
) -> Option<()> {
    let generator = match state {
        Some(generator) if generator.remaining >= mask.len() => generator,
        _ => state.insert(MaskGenerator {
            rng: seed()?,
            remaining: RESEED_AFTER_BYTES,
        }),
    };
    generator.rng.fill_bytes(mask);
    generator.remaining = generator.remaining.saturating_sub(mask.len());
    Some(())
}

/// Lets a test make the operating system's random source fail on its own
/// thread.
#[cfg(test)]
pub(crate) mod failure {
    use std::cell::Cell;

    thread_local! {
        static FAILS: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn os_source_fails() -> bool {
        FAILS.with(Cell::get)
    }

    /// Makes the source fail on this thread until dropped, and drops this
    /// thread's mask generator so the next mask needs a seed. A
    /// current-thread runtime (`#[tokio::test]`) keeps a whole request on
    /// the test's thread.
    pub(crate) struct FailingOsSource(());

    impl FailingOsSource {
        pub(crate) fn start() -> Self {
            super::MASK_GENERATOR.with(|state| state.replace(None));
            FAILS.with(|fails| fails.set(true));
            Self(())
        }
    }

    impl Drop for FailingOsSource {
        fn drop(&mut self) {
            FAILS.with(|fails| fails.set(false));
        }
    }
}

#[cfg(test)]
#[path = "csrf_random_tests.rs"]
mod tests;
