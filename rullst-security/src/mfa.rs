use crate::telemetry::SecurityStore;
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha1 = Hmac<Sha1>;

/// Base32 character set for RFC 4648 encoding
const BASE32_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Generates a random Base32 encoded 160-bit (20 byte) TOTP secret key.
pub fn generate_mfa_secret() -> String {
    let mut secret = String::with_capacity(32);
    for _ in 0..32 {
        let idx = (rand::random::<u8>() as usize) % BASE32_ALPHABET.len();
        secret.push(BASE32_ALPHABET[idx] as char);
    }
    secret
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
    let mut mac = HmacSha1::new_from_slice(secret_bytes)
        .unwrap_or_else(|_| HmacSha1::new_from_slice(b"mfa_fallback_key").unwrap());
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
    let Some(secret_bytes) = decode_base32(base32_secret) else {
        return false;
    };
    let Ok(provided_code) = code.trim().parse::<u32>() else {
        return false;
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let current_counter = now / 30;

    // Check current, prev (-1), and next (+1) 30s windows
    for delta in -1..=1 {
        let target_counter = (current_counter as i64 + delta) as u64;
        let generated = generate_totp_at_counter(&secret_bytes, target_counter);
        if generated == provided_code {
            SecurityStore::global().inc_mfa_verifications();
            return true;
        }
    }

    false
}

/// Builds an `otpauth://` URI string suitable for generating QR codes in authenticator apps.
pub fn build_otpauth_uri(issuer: &str, account_name: &str, base32_secret: &str) -> String {
    let clean_issuer = rullst_core::html::escape_str(issuer);
    let clean_account = rullst_core::html::escape_str(account_name);
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits=6&period=30",
        clean_issuer, clean_account, base32_secret, clean_issuer
    )
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
        assert!(!verify_totp_code(&secret, "000000"));
    }

    #[test]
    fn test_otpauth_uri_builder() {
        let uri = build_otpauth_uri("RullstApp", "user@example.com", "JBSWY3DPEHPK3PXP");
        assert!(
            uri.starts_with("otpauth://totp/RullstApp:user@example.com?secret=JBSWY3DPEHPK3PXP")
        );
    }
}
