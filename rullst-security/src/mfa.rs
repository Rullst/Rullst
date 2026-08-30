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

/// Verifies a 6-digit TOTP code with time drift window tolerance (+-1 window).
pub fn verify_totp_code(base32_secret: &str, code: &str) -> bool {
    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let Some(secret_bytes) = decode_base32(base32_secret) else {
        return false;
    };
    if secret_bytes.len() < MIN_TOTP_SECRET_BYTES {
        return false;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let current_counter = now / 30;

    let counters = [
        current_counter.checked_sub(1),
        Some(current_counter),
        current_counter.checked_add(1),
    ];
    for target_counter in counters.into_iter().flatten() {
        let generated = generate_totp_at_counter(&secret_bytes, target_counter);
        let expected = format!("{generated:06}");
        if bool::from(expected.as_bytes().ct_eq(code.as_bytes())) {
            SecurityStore::global().inc_mfa_verifications();
            return true;
        }
    }

    false
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
