//! Subject-bound single-use recovery-code verifiers for MFA recovery flows.

use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

const CODE_ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const CODE_CHARACTERS: usize = 16;
const MIN_PEPPER_BYTES: usize = 32;
const MAX_CODES: usize = 20;
const MAX_SUBJECT_BYTES: usize = 256;
const DOMAIN: &[u8] = b"rullst.recovery-code.v1";

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryCodeError {
    #[error("invalid recovery-code configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("recovery-code pepper must contain at least 32 strong bytes")]
    WeakPepper,
    #[error("recovery-code verifier could not be initialized")]
    CryptoInitialization,
}

/// Persist this verifier, never the corresponding plaintext recovery code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecoveryCodeVerifier {
    version: u8,
    salt: [u8; 16],
    digest: [u8; 32],
}

impl RecoveryCodeVerifier {
    pub fn version(&self) -> u8 {
        self.version
    }
}

/// Plaintext codes and their storage-safe verifiers returned only at enrollment.
///
/// The plaintext vector is zeroized when this value is dropped. Render it once,
/// then persist only the verifier vector.
pub struct GeneratedRecoveryCodes {
    plaintext: Vec<String>,
    verifiers: Vec<RecoveryCodeVerifier>,
}

impl GeneratedRecoveryCodes {
    pub fn plaintext(&self) -> &[String] {
        &self.plaintext
    }

    pub fn verifiers(&self) -> &[RecoveryCodeVerifier] {
        &self.verifiers
    }

    pub fn into_parts(mut self) -> (Vec<String>, Vec<RecoveryCodeVerifier>) {
        (
            std::mem::take(&mut self.plaintext),
            std::mem::take(&mut self.verifiers),
        )
    }
}

impl Drop for GeneratedRecoveryCodes {
    fn drop(&mut self) {
        self.plaintext.zeroize();
    }
}

/// Generates 80-bit recovery codes bound to one canonical application subject.
pub fn generate_recovery_codes(
    subject: impl Into<String>,
    count: usize,
    pepper: &[u8],
) -> Result<GeneratedRecoveryCodes, RecoveryCodeError> {
    let subject = subject.into();
    validate_configuration(&subject, count, pepper)?;
    let mut plaintext = Vec::with_capacity(count);
    let mut verifiers = Vec::with_capacity(count);
    for _ in 0..count {
        let code = generate_code();
        let normalized_code =
            normalize_code(&code).ok_or(RecoveryCodeError::CryptoInitialization)?;
        let mut salt = [0_u8; 16];
        rand::fill(&mut salt);
        let digest = recovery_digest(&subject, &normalized_code, &salt, pepper)?;
        plaintext.push(code);
        verifiers.push(RecoveryCodeVerifier {
            version: 1,
            salt,
            digest,
        });
    }
    Ok(GeneratedRecoveryCodes {
        plaintext,
        verifiers,
    })
}

/// Verifies without consuming. Prefer [`consume_recovery_code`] in login flows.
pub fn verify_recovery_code(
    subject: &str,
    code: &str,
    verifier: &RecoveryCodeVerifier,
    pepper: &[u8],
) -> Result<bool, RecoveryCodeError> {
    validate_subject_and_pepper(subject, pepper)?;
    if verifier.version != 1 {
        return Ok(false);
    }
    let Some(code) = normalize_code(code) else {
        return Ok(false);
    };
    let candidate = recovery_digest(subject, &code, &verifier.salt, pepper)?;
    Ok(bool::from(candidate.ct_eq(&verifier.digest)))
}

/// Removes one matching verifier after scanning every candidate.
///
/// Callers backed by a database must perform the read, match and delete in one
/// transaction or equivalent compare-and-delete operation to retain single-use
/// semantics across processes.
pub fn consume_recovery_code(
    subject: &str,
    code: &str,
    verifiers: &mut Vec<RecoveryCodeVerifier>,
    pepper: &[u8],
) -> Result<bool, RecoveryCodeError> {
    validate_subject_and_pepper(subject, pepper)?;
    let Some(code) = normalize_code(code) else {
        return Ok(false);
    };
    let mut matched = None;
    for (index, verifier) in verifiers.iter().enumerate() {
        let valid_version = u8::from(verifier.version == 1);
        let candidate = recovery_digest(subject, &code, &verifier.salt, pepper)?;
        let digest_matches = candidate.ct_eq(&verifier.digest).unwrap_u8();
        if (valid_version & digest_matches) == 1 && matched.is_none() {
            matched = Some(index);
        }
    }
    if let Some(index) = matched {
        verifiers.remove(index);
        Ok(true)
    } else {
        Ok(false)
    }
}

fn validate_configuration(
    subject: &str,
    count: usize,
    pepper: &[u8],
) -> Result<(), RecoveryCodeError> {
    if count == 0 || count > MAX_CODES {
        return Err(RecoveryCodeError::InvalidConfiguration("count"));
    }
    validate_subject_and_pepper(subject, pepper)
}

fn validate_subject_and_pepper(subject: &str, pepper: &[u8]) -> Result<(), RecoveryCodeError> {
    if subject.is_empty()
        || subject.trim() != subject
        || subject.len() > MAX_SUBJECT_BYTES
        || subject.chars().any(char::is_control)
    {
        return Err(RecoveryCodeError::InvalidConfiguration("subject"));
    }
    let mut distinct = [false; 256];
    for byte in pepper {
        distinct[usize::from(*byte)] = true;
    }
    if pepper.len() < MIN_PEPPER_BYTES || distinct.iter().filter(|present| **present).count() < 12 {
        return Err(RecoveryCodeError::WeakPepper);
    }
    Ok(())
}

fn generate_code() -> String {
    let mut compact = [0_u8; CODE_CHARACTERS];
    for character in &mut compact {
        *character = CODE_ALPHABET[usize::from(rand::random::<u8>()) % CODE_ALPHABET.len()];
    }
    let mut formatted = String::with_capacity(CODE_CHARACTERS + 3);
    for (index, byte) in compact.into_iter().enumerate() {
        if index > 0 && index % 4 == 0 {
            formatted.push('-');
        }
        formatted.push(char::from(byte));
    }
    formatted
}

fn normalize_code(code: &str) -> Option<String> {
    let compact = code
        .bytes()
        .filter(|byte| *byte != b'-')
        .map(|byte| byte.to_ascii_uppercase())
        .collect::<Vec<_>>();
    if compact.len() != CODE_CHARACTERS || compact.iter().any(|byte| !CODE_ALPHABET.contains(byte))
    {
        return None;
    }
    String::from_utf8(compact).ok()
}

fn recovery_digest(
    subject: &str,
    normalized_code: &str,
    salt: &[u8; 16],
    pepper: &[u8],
) -> Result<[u8; 32], RecoveryCodeError> {
    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(pepper)
        .map_err(|_| RecoveryCodeError::CryptoInitialization)?;
    mac.update(DOMAIN);
    mac.update(&(subject.len() as u64).to_be_bytes());
    mac.update(subject.as_bytes());
    mac.update(salt);
    mac.update(normalized_code.as_bytes());
    Ok(mac.finalize().into_bytes().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEPPER: &[u8] = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // TM-AUTH-07: recovery codes are subject-bound, stored as verifiers and
    // removed on the first successful use.
    #[test]
    fn codes_are_subject_bound_single_use_and_do_not_store_plaintext() {
        let batch = generate_recovery_codes("learner-7", 8, PEPPER).expect("recovery batch");
        assert_eq!(batch.plaintext().len(), 8);
        assert_eq!(batch.verifiers().len(), 8);
        let code = batch.plaintext()[0].clone();
        let serialized = serde_json::to_string(batch.verifiers()).expect("serialized verifiers");
        assert!(!serialized.contains(&code));
        assert!(
            verify_recovery_code("learner-7", &code, &batch.verifiers()[0], PEPPER)
                .expect("correct subject")
        );
        assert!(
            !verify_recovery_code("learner-8", &code, &batch.verifiers()[0], PEPPER)
                .expect("wrong subject")
        );

        let (_, mut verifiers) = batch.into_parts();
        assert!(
            consume_recovery_code("learner-7", &code, &mut verifiers, PEPPER)
                .expect("first consume")
        );
        assert!(
            !consume_recovery_code("learner-7", &code, &mut verifiers, PEPPER)
                .expect("replay consume")
        );
        assert_eq!(verifiers.len(), 7);
    }

    #[test]
    fn configuration_format_pepper_and_tampering_fail_closed() {
        assert!(generate_recovery_codes("learner", 0, PEPPER).is_err());
        assert!(generate_recovery_codes("learner", 21, PEPPER).is_err());
        assert!(generate_recovery_codes(" learner", 8, PEPPER).is_err());
        assert!(generate_recovery_codes("learner", 8, b"weak").is_err());

        let batch = generate_recovery_codes("learner", 1, PEPPER).expect("recovery batch");
        let lowercase = batch.plaintext()[0].to_ascii_lowercase();
        assert!(
            verify_recovery_code("learner", &lowercase, &batch.verifiers()[0], PEPPER)
                .expect("normalized code")
        );
        assert!(
            !verify_recovery_code(
                "learner",
                "AAAA-AAAA-AAAA-AAAA",
                &batch.verifiers()[0],
                PEPPER,
            )
            .expect("wrong code")
        );
        assert!(
            !verify_recovery_code("learner", "not a code", &batch.verifiers()[0], PEPPER)
                .expect("invalid format")
        );
    }
}
