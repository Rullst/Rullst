use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, Generate, Nonce, Payload},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use zeroize::Zeroizing;

use super::{
    DocumentRecoveryBinding, DocumentRecoveryError, DocumentRecoveryKey, DocumentRecoveryPolicy,
    EncryptedDocumentSnapshot, MAX_KEY_ID_BYTES, operation::StoredSnapshot, valid_portable_label,
};

const PREFIX: &str = "RULLST-ORM-SNAPSHOT";
const VERSION: &str = "v1";
const AAD_DOMAIN: &[u8] = b"rullst-orm:document-recovery:v1";
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;

pub(super) fn seal(
    snapshot: &StoredSnapshot,
    key: &DocumentRecoveryKey,
    binding: &DocumentRecoveryBinding,
    policy: DocumentRecoveryPolicy,
) -> Result<EncryptedDocumentSnapshot, DocumentRecoveryError> {
    let plaintext = Zeroizing::new(
        serde_json::to_vec(snapshot).map_err(|_| DocumentRecoveryError::InvalidPayload)?,
    );
    if plaintext.len() > policy.max_snapshot_bytes() {
        return Err(DocumentRecoveryError::CapacityExceeded);
    }
    let nonce = Nonce::<Aes256Gcm>::try_generate()
        .map_err(|_| DocumentRecoveryError::RandomnessUnavailable)?;
    let aad = authenticated_data(key.key_id(), binding);
    let ciphertext = key
        .cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext.as_ref(),
                aad: &aad,
            },
        )
        .map_err(|_| DocumentRecoveryError::EncryptionFailed)?;
    let envelope = format!(
        "{PREFIX}:{VERSION}:{}:{}:{}",
        key.key_id(),
        URL_SAFE_NO_PAD.encode(nonce),
        URL_SAFE_NO_PAD.encode(ciphertext)
    );
    validate_envelope(&envelope, policy)?;
    Ok(EncryptedDocumentSnapshot(envelope))
}

pub(super) fn open(
    snapshot: &EncryptedDocumentSnapshot,
    key: &DocumentRecoveryKey,
    binding: &DocumentRecoveryBinding,
    policy: DocumentRecoveryPolicy,
) -> Result<StoredSnapshot, DocumentRecoveryError> {
    let parsed = parse_envelope(&snapshot.0, policy)?;
    if parsed.key_id != key.key_id() {
        return Err(DocumentRecoveryError::KeyIdMismatch);
    }
    let nonce = URL_SAFE_NO_PAD
        .decode(parsed.nonce)
        .map_err(|_| DocumentRecoveryError::InvalidEnvelope)?;
    let nonce = Nonce::<Aes256Gcm>::try_from(nonce.as_slice())
        .map_err(|_| DocumentRecoveryError::InvalidEnvelope)?;
    let ciphertext = URL_SAFE_NO_PAD
        .decode(parsed.ciphertext)
        .map_err(|_| DocumentRecoveryError::InvalidEnvelope)?;
    if ciphertext.len() > policy.max_snapshot_bytes().saturating_add(TAG_BYTES) {
        return Err(DocumentRecoveryError::InvalidEnvelope);
    }
    let aad = authenticated_data(key.key_id(), binding);
    let plaintext = Zeroizing::new(
        key.cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| DocumentRecoveryError::AuthenticationFailed)?,
    );
    if plaintext.len() > policy.max_snapshot_bytes() {
        return Err(DocumentRecoveryError::InvalidEnvelope);
    }
    serde_json::from_slice(&plaintext).map_err(|_| DocumentRecoveryError::InvalidPayload)
}

pub(super) fn validate_envelope(
    envelope: &str,
    policy: DocumentRecoveryPolicy,
) -> Result<(), DocumentRecoveryError> {
    parse_envelope(envelope, policy).map(|_| ())
}

struct ParsedEnvelope<'a> {
    key_id: &'a str,
    nonce: &'a str,
    ciphertext: &'a str,
}

fn parse_envelope(
    envelope: &str,
    policy: DocumentRecoveryPolicy,
) -> Result<ParsedEnvelope<'_>, DocumentRecoveryError> {
    if envelope.len() > max_envelope_bytes(policy)? {
        return Err(DocumentRecoveryError::InvalidEnvelope);
    }
    let mut parts = envelope.split(':');
    let prefix = parts.next().ok_or(DocumentRecoveryError::InvalidEnvelope)?;
    let version = parts.next().ok_or(DocumentRecoveryError::InvalidEnvelope)?;
    let key_id = parts.next().ok_or(DocumentRecoveryError::InvalidEnvelope)?;
    let nonce = parts.next().ok_or(DocumentRecoveryError::InvalidEnvelope)?;
    let ciphertext = parts.next().ok_or(DocumentRecoveryError::InvalidEnvelope)?;
    if prefix != PREFIX
        || parts.next().is_some()
        || !valid_portable_label(key_id, MAX_KEY_ID_BYTES)
        || nonce.is_empty()
        || ciphertext.is_empty()
    {
        return Err(DocumentRecoveryError::InvalidEnvelope);
    }
    if version != VERSION {
        return Err(DocumentRecoveryError::UnsupportedVersion);
    }
    let nonce_bytes = URL_SAFE_NO_PAD
        .decode(nonce)
        .map_err(|_| DocumentRecoveryError::InvalidEnvelope)?;
    let ciphertext_bytes = decoded_len(ciphertext).ok_or(DocumentRecoveryError::InvalidEnvelope)?;
    if nonce_bytes.len() != NONCE_BYTES
        || !valid_base64url(ciphertext)
        || ciphertext_bytes < TAG_BYTES
        || ciphertext_bytes > policy.max_snapshot_bytes().saturating_add(TAG_BYTES)
    {
        return Err(DocumentRecoveryError::InvalidEnvelope);
    }
    Ok(ParsedEnvelope {
        key_id,
        nonce,
        ciphertext,
    })
}

fn max_envelope_bytes(policy: DocumentRecoveryPolicy) -> Result<usize, DocumentRecoveryError> {
    encoded_len(policy.max_snapshot_bytes().saturating_add(TAG_BYTES))
        .and_then(|ciphertext| ciphertext.checked_add(256))
        .ok_or(DocumentRecoveryError::InvalidPolicy)
}

fn encoded_len(bytes: usize) -> Option<usize> {
    bytes.checked_add(2)?.checked_div(3)?.checked_mul(4)
}

fn decoded_len(encoded: &str) -> Option<usize> {
    let complete = encoded.len().checked_div(4)?.checked_mul(3)?;
    let tail = match encoded.len() % 4 {
        0 => 0,
        2 => 1,
        3 => 2,
        _ => return None,
    };
    complete.checked_add(tail)
}

fn valid_base64url(encoded: &str) -> bool {
    if !encoded
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return false;
    }
    let Some(last) = encoded
        .as_bytes()
        .last()
        .and_then(|byte| base64url_value(*byte))
    else {
        return false;
    };
    match encoded.len() % 4 {
        0 => true,
        2 => last & 0b00_1111 == 0,
        3 => last & 0b00_0011 == 0,
        _ => false,
    }
}

fn base64url_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

fn authenticated_data(key_id: &str, binding: &DocumentRecoveryBinding) -> Vec<u8> {
    let fields = [
        key_id.as_bytes(),
        binding.application_namespace().as_bytes(),
        binding.collection().as_str().as_bytes(),
    ];
    let mut aad = Vec::with_capacity(
        AAD_DOMAIN.len() + 3 * 8 + fields.iter().map(|v| v.len()).sum::<usize>(),
    );
    aad.extend_from_slice(AAD_DOMAIN);
    for field in fields {
        aad.extend_from_slice(&(field.len() as u64).to_be_bytes());
        aad.extend_from_slice(field);
    }
    aad
}

#[cfg(test)]
mod tests {
    use super::valid_base64url;

    #[test]
    fn base64url_shape_requires_canonical_unpadded_tail_bits() {
        assert!(valid_base64url("AA"));
        assert!(valid_base64url("AAA"));
        assert!(valid_base64url("AAAA"));
        assert!(!valid_base64url(""));
        assert!(!valid_base64url("A"));
        assert!(!valid_base64url("AB"));
        assert!(!valid_base64url("AAB"));
        assert!(!valid_base64url("AA="));
    }
}
