use crate::telemetry::SecurityStore;
use hmac::{Hmac, KeyInit, Mac};
use qrcode::{QrCode, render::svg};
use rand::{TryRng, rngs::SysRng};
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

type HmacSha1 = Hmac<Sha1>;

/// Base32 character set for RFC 4648 encoding
const BASE32_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
/// Minimum decoded TOTP secret length recommended for HMAC-SHA1.
pub const MIN_TOTP_SECRET_BYTES: usize = 20;
const MAX_MFA_LABEL_BYTES: usize = 256;

/// Generates a random Base32 encoded 160-bit TOTP secret from the OS RNG.
pub fn try_generate_mfa_secret() -> Result<String, crate::SecurityError> {
    let mut entropy = [0_u8; 32];
    SysRng.try_fill_bytes(&mut entropy).map_err(|_| {
        crate::SecurityError::General("operating-system randomness is unavailable".to_string())
    })?;
    Ok(entropy
        .iter()
        .map(|byte| BASE32_ALPHABET[(byte & 31) as usize] as char)
        .collect())
}

/// Generates a random Base32 encoded 160-bit (20 byte) TOTP secret key.
pub fn generate_mfa_secret() -> String {
    try_generate_mfa_secret().unwrap_or_default()
}

/// Decodes a Base32 string into a raw byte vector.
pub fn decode_base32(b32: &str) -> Option<Vec<u8>> {
    let clean = b32.trim().to_uppercase();
    let mut bits = 0u32;
    let mut num_bits = 0;
    let mut out = Vec::new();

    for ch in clean.chars() {
        if ch == '=' {
            break;
        }
        let val = match ch {
            'A'..='Z' => (ch as u32) - ('A' as u32),
            '2'..='7' => (ch as u32) - ('2' as u32) + 26,
            _ => return None,
        };
        bits = (bits << 5) | val;
        num_bits += 5;
        if num_bits >= 8 {
            num_bits -= 8;
            out.push((bits >> num_bits) as u8);
        }
    }

    Some(out)
}

/// Computes an RFC 6238 6-digit TOTP code for a secret at a specific counter step.
pub fn generate_totp_at_counter(secret_bytes: &[u8], counter: u64) -> u32 {
    if secret_bytes.len() < MIN_TOTP_SECRET_BYTES {
        return 0;
    }
    let mut mac = match HmacSha1::new_from_slice(secret_bytes) {
        Ok(mac) => mac,
        Err(_) => return 0,
    };
    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();

    let offset = (result[result.len() - 1] & 0x0f) as usize;
    let binary = ((result[offset] & 0x7f) as u32) << 24
        | (result[offset + 1] as u32) << 16
        | (result[offset + 2] as u32) << 8
        | (result[offset + 3] as u32);

    binary % 1_000_000
}

/// Computes the current 6-digit TOTP code for a Base32 secret.
pub fn generate_totp_code(base32_secret: &str) -> Option<String> {
    let secret_bytes = decode_base32(base32_secret)?;
    if secret_bytes.len() < MIN_TOTP_SECRET_BYTES {
        return None;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let counter = now / 30;
    let code = generate_totp_at_counter(&secret_bytes, counter);
    Some(format!("{:06}", code))
}

/// TOTP time-step length in seconds (RFC 6238 default).
const TOTP_STEP_SECONDS: u64 = 30;

/// Verifies a 6-digit TOTP code with time drift window tolerance (+-1 window).
///
/// # Replay
///
/// This function is stateless: it accepts any code for the previous, current
/// or next 30-second step, so the same code verifies again for up to about 90
/// seconds. It does not implement the RFC 6238 section 5.2 rule that a
/// verifier must not accept the same one-time password twice. Use
/// [`verify_totp_step_after`] with the last accepted step persisted per
/// secret to reject replays.
pub fn verify_totp_code(base32_secret: &str, code: &str) -> bool {
    verify_totp_step(base32_secret, code).is_some()
}

/// Verifies a 6-digit TOTP code (+-1 step) and returns the matched time step.
///
/// The step is `unix_seconds / 30`. Like [`verify_totp_code`] this is
/// stateless and does not reject replays; pass the returned step to
/// [`verify_totp_step_after`] on the next verification.
pub fn verify_totp_step(base32_secret: &str, code: &str) -> Option<u64> {
    verify_totp_step_at(base32_secret, code, current_totp_step(), None)
}

/// Verifies a 6-digit TOTP code (+-1 step) and rejects replays (RFC 6238 section 5.2).
///
/// Only a step strictly greater than `last_accepted_step` can match, so a code
/// is accepted at most once and older codes are rejected after a newer one.
/// Pass `None` when no code has been accepted for this secret. On success the
/// caller must persist the returned step atomically before admitting the
/// login, for example with a conditional
/// `UPDATE ... SET last_step = $step WHERE last_step IS NULL OR last_step < $step`
/// that affects exactly one row; otherwise two concurrent requests can both
/// accept the same code.
///
/// ```rust
/// use rullst_security::mfa::{generate_mfa_secret, generate_totp_code, verify_totp_step_after};
///
/// let secret = generate_mfa_secret();
/// let code = generate_totp_code(&secret).expect("valid secret");
/// // Stored with the secret and updated atomically in production.
/// let mut last_accepted_step = None;
///
/// let step = verify_totp_step_after(&secret, &code, last_accepted_step).expect("fresh code");
/// last_accepted_step = Some(step);
///
/// assert_eq!(verify_totp_step_after(&secret, &code, last_accepted_step), None);
/// ```
pub fn verify_totp_step_after(
    base32_secret: &str,
    code: &str,
    last_accepted_step: Option<u64>,
) -> Option<u64> {
    verify_totp_step_at(base32_secret, code, current_totp_step(), last_accepted_step)
}

fn current_totp_step() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / TOTP_STEP_SECONDS
}

/// Returns the lowest step in `current_step` +-1 that is newer than
/// `last_accepted_step` and whose code equals `code` in constant time.
fn verify_totp_step_at(
    base32_secret: &str,
    code: &str,
    current_step: u64,
    last_accepted_step: Option<u64>,
) -> Option<u64> {
    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let secret_bytes = decode_base32(base32_secret)?;
    if secret_bytes.len() < MIN_TOTP_SECRET_BYTES {
        return None;
    }

    let steps = [
        current_step.checked_sub(1),
        Some(current_step),
        current_step.checked_add(1),
    ];
    let matched = steps
        .into_iter()
        .flatten()
        .filter(|step| last_accepted_step.is_none_or(|last| *step > last))
        .find(|&step| {
            let expected = format!("{:06}", generate_totp_at_counter(&secret_bytes, step));
            bool::from(expected.as_bytes().ct_eq(code.as_bytes()))
        })?;
    SecurityStore::global().inc_mfa_verifications();
    Some(matched)
}

/// Builds an `otpauth://` URI string suitable for generating QR codes in authenticator apps.
pub fn build_otpauth_uri(issuer: &str, account_name: &str, base32_secret: &str) -> String {
    let encoded_issuer = urlencoding::encode(issuer);
    let encoded_account = urlencoding::encode(account_name);
    let encoded_secret = urlencoding::encode(base32_secret);
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits=6&period=30",
        encoded_issuer, encoded_account, encoded_secret, encoded_issuer
    )
}

/// Builds a self-contained SVG QR code for authenticator enrollment.
///
/// Labels are bounded and the TOTP secret must decode to at least 160 bits.
pub fn build_mfa_qr_svg(
    issuer: &str,
    account_name: &str,
    base32_secret: &str,
) -> Result<String, crate::SecurityError> {
    if issuer.trim().is_empty()
        || account_name.trim().is_empty()
        || issuer.len() > MAX_MFA_LABEL_BYTES
        || account_name.len() > MAX_MFA_LABEL_BYTES
    {
        return Err(crate::SecurityError::General(
            "MFA issuer and account labels must be non-empty and at most 256 bytes".to_string(),
        ));
    }
    let secret = decode_base32(base32_secret).ok_or_else(|| {
        crate::SecurityError::General("MFA secret is not valid Base32".to_string())
    })?;
    if secret.len() < MIN_TOTP_SECRET_BYTES {
        return Err(crate::SecurityError::General(
            "MFA secret must contain at least 160 bits".to_string(),
        ));
    }

    let uri = build_otpauth_uri(issuer.trim(), account_name.trim(), base32_secret.trim());
    let code = QrCode::new(uri.as_bytes()).map_err(|_| {
        crate::SecurityError::General("MFA enrollment data is too large for a QR code".to_string())
    })?;
    Ok(code
        .render::<svg::Color<'_>>()
        .min_dimensions(256, 256)
        .dark_color(svg::Color("#020617"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mfa_secret_generation() {
        let secret = generate_mfa_secret();
        assert_eq!(secret.len(), 32);
        assert!(decode_base32(&secret).is_some());
    }

    #[test]
    fn test_totp_verification() {
        let secret = generate_mfa_secret();
        let code = generate_totp_code(&secret).expect("TOTP generation failed");
        assert_eq!(code.len(), 6);
        assert!(verify_totp_code(&secret, &code));
        let wrong_code = if code == "000000" { "000001" } else { "000000" };
        assert!(!verify_totp_code(&secret, wrong_code));
    }

    #[test]
    fn test_otpauth_uri_builder() {
        let uri = build_otpauth_uri(
            "Rullst & Co",
            "user+ops@example.com/admin",
            "JBSWY3DPEHPK3PXP",
        );
        assert!(
            uri.starts_with(
                "otpauth://totp/Rullst%20%26%20Co:user%2Bops%40example.com%2Fadmin?secret=JBSWY3DPEHPK3PXP"
            )
        );
        assert!(uri.contains("issuer=Rullst%20%26%20Co"));
        assert!(!uri.contains("&amp;"));
    }

    #[test]
    fn totp_requires_exactly_six_ascii_digits() {
        let secret = generate_mfa_secret();
        assert!(!verify_totp_code(&secret, "12345"));
        assert!(!verify_totp_code(&secret, "0123456"));
        assert!(!verify_totp_code(&secret, " 12345"));
        assert!(!verify_totp_code(&secret, "１２３４５６"));
        assert!(generate_totp_code("JBSWY3DPEHPK3PXP").is_none());
        assert!(!verify_totp_code("JBSWY3DPEHPK3PXP", "000000"));
    }

    #[test]
    fn replayed_code_is_rejected_once_its_step_is_recorded() {
        // RFC 6238 Appendix B SHA-1 secret; a fixed secret keeps the test deterministic.
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        let bytes = decode_base32(secret).unwrap();
        assert_eq!(generate_totp_at_counter(&bytes, 1), 287_082);
        let step = 57_000_000;
        let code_at = |step| format!("{:06}", generate_totp_at_counter(&bytes, step));
        let code = code_at(step);

        // The stateless check accepts the same code on every attempt.
        assert_eq!(verify_totp_step_at(secret, &code, step, None), Some(step));
        assert_eq!(verify_totp_step_at(secret, &code, step, None), Some(step));
        // With the accepted step supplied, the second attempt is rejected,
        // including after the clock moves on within the drift window.
        assert_eq!(verify_totp_step_at(secret, &code, step, Some(step)), None);
        assert_eq!(
            verify_totp_step_at(secret, &code, step + 1, Some(step)),
            None
        );
        // A newer code is still accepted; an older one is not.
        assert_eq!(
            verify_totp_step_at(secret, &code_at(step + 1), step, Some(step)),
            Some(step + 1)
        );
        assert_eq!(
            verify_totp_step_at(secret, &code_at(step - 1), step, Some(step)),
            None
        );
        // Codes outside the +-1 window never match.
        assert_eq!(
            verify_totp_step_at(secret, &code_at(step + 2), step, None),
            None
        );
    }

    #[test]
    fn public_step_api_matches_the_boolean_verifier_and_blocks_replay() {
        let secret = generate_mfa_secret();
        let code = generate_totp_code(&secret).unwrap();
        let step = verify_totp_step(&secret, &code).expect("fresh code");
        assert!(verify_totp_code(&secret, &code));
        assert_eq!(verify_totp_step_after(&secret, &code, None), Some(step));
        assert_eq!(verify_totp_step_after(&secret, &code, Some(step)), None);

        assert_eq!(verify_totp_step(&secret, "12345"), None);
        assert_eq!(verify_totp_step("JBSWY3DPEHPK3PXP", "000000"), None);
        assert_eq!(verify_totp_step_after("not base32!", "000000", None), None);
    }

    #[test]
    fn enrollment_qr_is_real_svg_and_rejects_weak_inputs() {
        let secret = generate_mfa_secret();
        let svg = build_mfa_qr_svg("Rullst", "user@example.com", &secret)
            .expect("valid enrollment data should render");
        assert!(svg.starts_with("<?xml"));
        assert!(svg.contains("<svg"));
        assert!(svg.contains("#020617"));
        assert!(build_mfa_qr_svg("", "user@example.com", &secret).is_err());
        assert!(build_mfa_qr_svg("Rullst", "user@example.com", "ABC").is_err());
    }
}
