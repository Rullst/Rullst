//! AES-256-GCM under a key that the application manages, outside the ORM's
//! configured key and keyring.

use super::{
    DEFAULT_KEY_ID, ENVELOPE_PREFIX, PrivacyError, decrypt_envelope, decrypt_legacy_aes_gcm,
    encrypt_with_context, parse_envelope, validate_key,
};

/// Encrypts `plaintext` with AES-256-GCM under a key that the application
/// manages itself, for example one derived from `APP_KEY`.
///
/// `key` is used as-is: its UTF-8 bytes are the AES-256 key, so it must be
/// exactly 32 bytes long (`key.len() == 32`). It is not decoded: unlike
/// `RULLST_ENCRYPTION_KEY`, a `base64:` or `hex:` prefix is part of the key.
/// Because the key is text, 32 printable characters carry fewer than 256 bits
/// (32 base64url characters carry 192, 32 hexadecimal digits 128): derive it
/// from a high-entropy secret with a key-derivation function or generate it
/// randomly, and never use a password as the key.
///
/// The result is Rullst's versioned envelope
/// `RULLST:v2:default:<nonce>:<ciphertext>`, both parts in unpadded base64url,
/// the ciphertext ending with the 16-byte authentication tag. Each call uses
/// a fresh random 96-bit nonce, so the same plaintext encrypts to a different
/// value every time (do not use the output for lookups or equality), and one
/// key should encrypt at most about 2^32 values. The associated context is
/// empty: the key ID `default` is authenticated, but no table, column or
/// record, so a value can be copied between rows without detection.
///
/// The application owns key storage and rotation: the envelope always names
/// the key ID `default`, and [`decrypt_aes_gcm`] uses only the key it is
/// given. To rotate, decrypt with the old key and encrypt again with the new
/// one; while values of both keys exist, try the new key and fall back to the
/// old one on [`PrivacyError::DecryptionFailed`].
///
/// For model fields, prefer `#[orm(encrypted)]` (which calls
/// [`encrypt_model_field`]) or [`SecretString`]: they use the configured
/// `RULLST_ENCRYPTION_KEY` and keyring, select the key by its ID, and
/// `#[orm(encrypted)]` binds each value to its table and column.
/// [`decrypt_model_field`] cannot read values from this function, nor
/// [`decrypt_aes_gcm`] values from [`encrypt_model_field`].
///
/// # Errors
///
/// [`PrivacyError::InvalidKeyLength`] when `key` is not exactly 32 bytes
/// long, [`PrivacyError::RandomnessUnavailable`] when the operating system
/// cannot provide a nonce and [`PrivacyError::EncryptionFailed`] when AES-GCM
/// fails.
///
/// # Example
///
/// ```
/// use rullst_orm::privacy::{decrypt_aes_gcm, encrypt_aes_gcm};
///
/// // A test key of exactly 32 bytes; derive a real one from a secret.
/// let key = "0123456789abcdefghijklmnopqrstuv";
/// let encrypted = encrypt_aes_gcm("refresh-token", key)?;
/// assert!(encrypted.starts_with("RULLST:v2:default:"));
/// assert_eq!(decrypt_aes_gcm(&encrypted, key)?, "refresh-token");
/// # Ok::<(), rullst_orm::privacy::PrivacyError>(())
/// ```
///
/// [`encrypt_model_field`]: super::encrypt_model_field
/// [`decrypt_model_field`]: super::decrypt_model_field
/// [`SecretString`]: super::SecretString
pub fn encrypt_aes_gcm(plaintext: &str, key: &str) -> Result<String, PrivacyError> {
    encrypt_with_context(plaintext, key.as_bytes(), DEFAULT_KEY_ID, b"")
}

/// Decrypts a value written by [`encrypt_aes_gcm`] under the same `key`.
///
/// `key` follows the rules of [`encrypt_aes_gcm`]: exactly 32 bytes, whose
/// UTF-8 bytes are the AES-256 key, with no decoding. A value that starts with
/// `RULLST:` is read as the versioned envelope with an empty associated
/// context; its key ID is authenticated but does not select the key. Any other
/// value is read as the pre-v12 format, standard padded base64 of the 12-byte
/// nonce followed by the ciphertext and its 16-byte tag, with no associated
/// data, so values written before v12 stay readable; encrypt them again to
/// move them to the envelope.
///
/// Values from [`encrypt_model_field`] and `#[orm(encrypted)]` are bound to
/// their table and column and fail here with
/// [`PrivacyError::DecryptionFailed`]; read them with
/// [`decrypt_model_field`].
///
/// # Errors
///
/// [`PrivacyError::InvalidKeyLength`] when `key` is not exactly 32 bytes
/// long; [`PrivacyError::DecryptionFailed`] for a wrong key or a modified
/// value; [`PrivacyError::InvalidEnvelope`],
/// [`PrivacyError::UnsupportedEnvelopeVersion`],
/// [`PrivacyError::InvalidKeyId`], [`PrivacyError::Base64Error`] or
/// [`PrivacyError::PayloadTooShort`] for a malformed value; and
/// [`PrivacyError::Utf8Error`] when the plaintext is not UTF-8.
///
/// [`encrypt_model_field`]: super::encrypt_model_field
/// [`decrypt_model_field`]: super::decrypt_model_field
pub fn decrypt_aes_gcm(encrypted: &str, key: &str) -> Result<String, PrivacyError> {
    let key_bytes = key.as_bytes();
    validate_key(key_bytes)?;
    if encrypted.starts_with(&format!("{ENVELOPE_PREFIX}:")) {
        let envelope = parse_envelope(encrypted)?;
        return decrypt_envelope(&envelope, key_bytes, b"");
    }

    decrypt_legacy_aes_gcm(encrypted, key_bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::privacy::{NONCE_LENGTH, TAG_LENGTH, model_context};
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    /// 32 bytes; the key of the fixed values below.
    const KEY: &str = "alpha4-explicit-key-0123456789ab";
    const OTHER_KEY: &str = "another-explicit-key-0123456789a";
    /// `encrypt_aes_gcm("google refresh token", KEY)` in the 13.0.0-alpha.4
    /// format, computed with an independent AES-256-GCM implementation: nonce
    /// bytes 1 to 12, associated data
    /// `rullst-orm:field-encryption:aes-256-gcm\0v2\0default\0`.
    const ALPHA4_ENVELOPE: &str =
        "RULLST:v2:default:AQIDBAUGBwgJCgsM:dZO-XyukR6VHK35T9ewaE9j36MdlF1QUCcLCyrWrlR1DKGQQ";
    /// "pre-v12 AI key" in the pre-v12 format: standard base64 of nonce bytes
    /// 21 to 32, then the ciphertext and tag, with no associated data.
    const PRE_V12: &str = "FRYXGBkaGxwdHh8gUbEESzxsWi/jaIb109m19V3+navGFkg9dtNNmvdg";

    fn rejected(result: Result<String, PrivacyError>) -> bool {
        matches!(result, Err(PrivacyError::DecryptionFailed(_)))
    }

    #[test]
    fn explicit_keys_round_trip_through_the_unbound_default_envelope() {
        let plaintext = "google refresh token";
        let first = encrypt_aes_gcm(plaintext, KEY).unwrap();
        let second = encrypt_aes_gcm(plaintext, KEY).unwrap();
        assert_ne!(first, second, "every call draws a fresh nonce");
        for encrypted in [&first, &second] {
            assert!(encrypted.starts_with("RULLST:v2:default:"), "{encrypted}");
            let envelope = parse_envelope(encrypted).unwrap();
            assert_eq!(envelope.key_id, "default");
            assert_eq!(envelope.nonce.len(), NONCE_LENGTH);
            assert_eq!(envelope.ciphertext.len(), plaintext.len() + TAG_LENGTH);
            assert_eq!(
                decrypt_envelope(&envelope, KEY.as_bytes(), b"").unwrap(),
                plaintext
            );
            assert_eq!(decrypt_aes_gcm(encrypted, KEY).unwrap(), plaintext);
        }
        let empty = encrypt_aes_gcm("", KEY).unwrap();
        assert_eq!(decrypt_aes_gcm(&empty, KEY).unwrap(), "");
    }

    #[test]
    fn the_key_is_its_raw_bytes_and_must_be_exactly_32_bytes_long() {
        // `base64:` and `hex:` prefixes are not decoded, and the length counts
        // bytes, not characters.
        let unicode = "é".repeat(16);
        let base64_text = format!("base64:{}", "A".repeat(25));
        let hex_text = format!("hex:{}", "0".repeat(28));
        for key in [unicode.as_str(), base64_text.as_str(), hex_text.as_str()] {
            assert_eq!(key.len(), 32);
            let encrypted = encrypt_aes_gcm("secret", key).unwrap();
            assert_eq!(decrypt_aes_gcm(&encrypted, key).unwrap(), "secret");
            assert!(rejected(decrypt_aes_gcm(&encrypted, KEY)));
        }

        let encrypted = encrypt_aes_gcm("secret", KEY).unwrap();
        let long = format!("{KEY}x");
        let wide = "é".repeat(32);
        for key in ["", &KEY[..31], long.as_str(), wide.as_str()] {
            assert_eq!(
                encrypt_aes_gcm("secret", key),
                Err(PrivacyError::InvalidKeyLength)
            );
            assert_eq!(
                decrypt_aes_gcm(&encrypted, key),
                Err(PrivacyError::InvalidKeyLength)
            );
        }
        // The key is checked before the value is read.
        assert_eq!(
            decrypt_aes_gcm("not base64!", "short"),
            Err(PrivacyError::InvalidKeyLength)
        );
    }

    #[test]
    fn a_wrong_key_a_modified_value_or_a_bound_value_fails_authentication() {
        let encrypted = encrypt_aes_gcm("secret", KEY).unwrap();
        assert!(rejected(decrypt_aes_gcm(&encrypted, OTHER_KEY)));

        let (head, body) = encrypted.rsplit_once(':').unwrap();
        let mut ciphertext = URL_SAFE_NO_PAD.decode(body).unwrap();
        ciphertext[0] ^= 1;
        let modified = format!("{head}:{}", URL_SAFE_NO_PAD.encode(ciphertext));
        assert!(rejected(decrypt_aes_gcm(&modified, KEY)));

        // Values bound to a table and column, as `encrypt_model_field` writes
        // them, are not readable here, and the reverse.
        let context = model_context("accounts", "token");
        let bound = encrypt_with_context("secret", KEY.as_bytes(), "default", &context).unwrap();
        assert!(rejected(decrypt_aes_gcm(&bound, KEY)));
        let envelope = parse_envelope(&encrypted).unwrap();
        assert!(rejected(decrypt_envelope(
            &envelope,
            KEY.as_bytes(),
            &context
        )));
    }

    #[test]
    fn alpha4_and_pre_v12_values_stay_readable() {
        assert_eq!(
            decrypt_aes_gcm(ALPHA4_ENVELOPE, KEY).unwrap(),
            "google refresh token"
        );
        assert_eq!(decrypt_aes_gcm(PRE_V12, KEY).unwrap(), "pre-v12 AI key");
        for value in [ALPHA4_ENVELOPE, PRE_V12] {
            assert!(rejected(decrypt_aes_gcm(value, OTHER_KEY)), "{value}");
        }

        // The key ID is authenticated, and only `v2` envelopes are read.
        let renamed = ALPHA4_ENVELOPE.replacen(":default:", ":rotated:", 1);
        assert!(rejected(decrypt_aes_gcm(&renamed, KEY)));
        assert_eq!(
            decrypt_aes_gcm(&ALPHA4_ENVELOPE.replacen(":v2:", ":v3:", 1), KEY),
            Err(PrivacyError::UnsupportedEnvelopeVersion("v3".to_string()))
        );
    }
}
