//! Per-response masking of the double-submit CSRF token (BREACH defence).
//!
//! The `rullst_csrf` cookie keeps one secret per browser. A page that
//! embedded that secret as it is would repeat the same bytes in every
//! response, so a compressed page that also reflects attacker-controlled
//! input would leak it byte by byte through its compressed length (BREACH).
//! Every value embedded in a response body is therefore
//! `mask || (token XOR mask)`, with a fresh mask from this thread's CSPRNG
//! (seeded and reseeded from the operating system, see `csrf_random`), in
//! URL-safe base64 without padding: it differs in every response and
//! unmasks to the cookie value.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use subtle::{Choice, ConstantTimeEq};

/// Longest `rullst_csrf` cookie value the middleware accepts.
const MAX_COOKIE_TOKEN_BYTES: usize = 128;

/// Longest masked value: a 128-byte mask and a 128-byte masked token in
/// base64 without padding.
pub(crate) const MAX_MASKED_TOKEN_BYTES: usize = 342;

/// Whether `token` is a value the `rullst_csrf` cookie may carry: 1 to 128
/// ASCII letters, digits, `-` or `_`.
pub(crate) fn valid_cookie_token(token: &[u8]) -> bool {
    (1..=MAX_COOKIE_TOKEN_BYTES).contains(&token.len())
        && token
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// Masks a `rullst_csrf` cookie value for one response.
///
/// Returns `mask || (token XOR mask)` in URL-safe base64 without padding,
/// with a fresh mask of the token's length from a CSPRNG (a ChaCha12
/// generator per thread, seeded and reseeded from the operating system's
/// random source), so two calls never return the same text. The CSRF
/// middleware accepts the result, like the cookie value itself, in the
/// `X-CSRF-Token` header or the `_token` form field of a request carrying
/// that cookie. [`CsrfToken::as_str`](super::CsrfToken::as_str) is already
/// masked; call this only for a value read from the cookie.
///
/// Returns `None` when `token` is not a valid cookie value (1 to 128 ASCII
/// letters, digits, `-` or `_`) or the operating system's random source
/// fails to seed that generator.
///
/// ```
/// let cookie = rullst_core::security::generate_csrf_token();
/// let first = rullst_core::security::mask_csrf_token(&cookie).expect("valid token");
/// let second = rullst_core::security::mask_csrf_token(&cookie).expect("valid token");
/// assert_ne!(first, second);
/// assert_eq!(first.len(), 86);
/// assert!(rullst_core::security::mask_csrf_token("not a token").is_none());
/// ```
pub fn mask_csrf_token(token: &str) -> Option<String> {
    let token = token.as_bytes();
    if !valid_cookie_token(token) {
        return None;
    }
    let mut masked = vec![0_u8; token.len() * 2];
    let (mask, cipher) = masked.split_at_mut(token.len());
    super::csrf_random::fill_mask(mask)?;
    for ((out, pad), byte) in cipher.iter_mut().zip(mask.iter()).zip(token) {
        *out = pad ^ byte;
    }
    Some(URL_SAFE_NO_PAD.encode(masked))
}

/// Whether `candidate`, from a request header or form field, proves the
/// `cookie` token: the cookie value itself, or a masked value that unmasks
/// to it. The token bytes are compared in constant time; only lengths,
/// which are public, decide earlier.
pub(crate) fn csrf_proof_matches(candidate: &[u8], cookie: &str) -> Choice {
    let cookie = cookie.as_bytes();
    if candidate.len() == cookie.len() {
        return candidate.ct_eq(cookie);
    }
    if Some(candidate.len()) != base64::encoded_len(cookie.len() * 2, false) {
        return Choice::from(0);
    }
    let Ok(decoded) = URL_SAFE_NO_PAD.decode(candidate) else {
        return Choice::from(0);
    };
    let (mask, cipher) = decoded.split_at(decoded.len() / 2);
    let token = mask
        .iter()
        .zip(cipher)
        .map(|(pad, byte)| pad ^ byte)
        .collect::<Vec<_>>();
    token.ct_eq(cookie)
}

#[cfg(test)]
#[path = "csrf_mask_tests.rs"]
mod tests;
