use crate::sentinel::SentinelError;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

const TOKEN_DOMAIN: &[u8] = b"rullst.sentinel.pow-token.v1";
const SUBJECT_DOMAIN: &[u8] = b"rullst.sentinel.pow-subject.v1";
const SOLUTION_DOMAIN: &[u8] = b"rullst.sentinel.pow-solution.v1";
pub(super) const TOKEN_VERSION: u8 = 1;
pub(super) const CHALLENGE_ID_BYTES: usize = 16;
pub(super) const SUBJECT_TAG_BYTES: usize = 16;
const MAC_BYTES: usize = 32;
const PAYLOAD_BYTES: usize = 1 + CHALLENGE_ID_BYTES + SUBJECT_TAG_BYTES + 8 + 8 + 1;
const TOKEN_BYTES: usize = PAYLOAD_BYTES + MAC_BYTES;
const MAX_TOKEN_TEXT_BYTES: usize = 128;

pub(super) struct ParsedToken {
    pub(super) version: u8,
    pub(super) challenge_id: [u8; CHALLENGE_ID_BYTES],
    pub(super) subject_tag: [u8; SUBJECT_TAG_BYTES],
    pub(super) issued_at_epoch: u64,
    pub(super) expires_at_epoch: u64,
    pub(super) difficulty_bits: u8,
}

impl ParsedToken {
    pub(super) fn from_bytes(bytes: &[u8; TOKEN_BYTES]) -> Result<Self, SentinelError> {
        let mut challenge_id = [0_u8; CHALLENGE_ID_BYTES];
        challenge_id.copy_from_slice(&bytes[1..17]);
        let mut subject_tag = [0_u8; SUBJECT_TAG_BYTES];
        subject_tag.copy_from_slice(&bytes[17..33]);
        let issued_at_epoch = u64::from_be_bytes(
            bytes[33..41]
                .try_into()
                .map_err(|_| SentinelError::InvalidToken)?,
        );
        let expires_at_epoch = u64::from_be_bytes(
            bytes[41..49]
                .try_into()
                .map_err(|_| SentinelError::InvalidToken)?,
        );
        Ok(Self {
            version: bytes[0],
            challenge_id,
            subject_tag,
            issued_at_epoch,
            expires_at_epoch,
            difficulty_bits: bytes[49],
        })
    }
}

pub(super) fn build_token(
    key: &[u8],
    challenge_id: [u8; CHALLENGE_ID_BYTES],
    subject_tag: [u8; SUBJECT_TAG_BYTES],
    issued_at_epoch: u64,
    expires_at_epoch: u64,
    difficulty_bits: u8,
) -> Result<String, SentinelError> {
    let mut bytes = Vec::with_capacity(TOKEN_BYTES);
    bytes.push(TOKEN_VERSION);
    bytes.extend_from_slice(&challenge_id);
    bytes.extend_from_slice(&subject_tag);
    bytes.extend_from_slice(&issued_at_epoch.to_be_bytes());
    bytes.extend_from_slice(&expires_at_epoch.to_be_bytes());
    bytes.push(difficulty_bits);
    let mut mac = new_mac(key)?;
    mac.update(TOKEN_DOMAIN);
    mac.update(&bytes);
    bytes.extend_from_slice(&mac.finalize().into_bytes());
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub(super) fn decode_token(token: &str) -> Result<[u8; TOKEN_BYTES], SentinelError> {
    if token.is_empty() || token.len() > MAX_TOKEN_TEXT_BYTES {
        return Err(SentinelError::InvalidToken);
    }
    URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| SentinelError::InvalidToken)?
        .try_into()
        .map_err(|_| SentinelError::InvalidToken)
}

pub(super) fn verify_token_mac(key: &[u8], bytes: &[u8; TOKEN_BYTES]) -> Result<(), SentinelError> {
    let mut mac = new_mac(key)?;
    mac.update(TOKEN_DOMAIN);
    mac.update(&bytes[..PAYLOAD_BYTES]);
    mac.verify_slice(&bytes[PAYLOAD_BYTES..])
        .map_err(|_| SentinelError::InvalidToken)
}

pub(super) fn subject_tag(
    key: &[u8],
    subject: &str,
) -> Result<[u8; SUBJECT_TAG_BYTES], SentinelError> {
    let mut mac = new_mac(key)?;
    mac.update(SUBJECT_DOMAIN);
    mac.update(&(subject.len() as u64).to_be_bytes());
    mac.update(subject.as_bytes());
    let digest = mac.finalize().into_bytes();
    let mut tag = [0_u8; SUBJECT_TAG_BYTES];
    tag.copy_from_slice(&digest[..SUBJECT_TAG_BYTES]);
    Ok(tag)
}

pub(super) fn proof_matches(bytes: &[u8; TOKEN_BYTES], nonce: u64, difficulty_bits: u8) -> bool {
    if !(8..=24).contains(&difficulty_bits) || bytes[49] != difficulty_bits {
        return false;
    }
    let mut hasher = Sha256::new();
    hasher.update(SOLUTION_DOMAIN);
    hasher.update(bytes);
    hasher.update(nonce.to_be_bytes());
    let digest: [u8; 32] = hasher.finalize().into();
    has_leading_zero_bits(&digest, difficulty_bits)
}

fn new_mac(key: &[u8]) -> Result<HmacSha256, SentinelError> {
    <HmacSha256 as KeyInit>::new_from_slice(key).map_err(|_| SentinelError::CryptoInitialization)
}

fn has_leading_zero_bits(digest: &[u8; 32], difficulty_bits: u8) -> bool {
    let full_bytes = usize::from(difficulty_bits / 8);
    let remaining_bits = difficulty_bits % 8;
    if digest[..full_bytes].iter().any(|byte| *byte != 0) {
        return false;
    }
    remaining_bits == 0 || digest[full_bytes] & (0xff << (8 - remaining_bits)) == 0
}
