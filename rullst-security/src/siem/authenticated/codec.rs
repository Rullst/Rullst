use super::{AuthenticatedSiemSpoolError, SiemKeyRing};
use crate::telemetry::{LiveSecurityEvent, SECURITY_EVENT_SCHEMA_VERSION};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

pub(super) const SPOOL_MAGIC: &[u8] = b"RULLST-SIEM-AUTH-SPOOL-V1\n";
const HMAC_DOMAIN: &[u8] = b"RULLST-SIEM-AUTH-FRAME\0V1";
const MAX_RECORD_BYTES: usize = 16 * 1024;
const LENGTH_HEX_BYTES: usize = 8;
const SEQUENCE_HEX_BYTES: usize = 16;
const KEY_LENGTH_HEX_BYTES: usize = 2;
const TAG_HEX_BYTES: usize = 64;

pub(super) struct DecodedSpool {
    pub(super) bytes: u64,
    pub(super) events: Vec<LiveSecurityEvent>,
    pub(super) last_tag: [u8; 32],
}

impl DecodedSpool {
    pub(super) fn empty() -> Self {
        Self {
            bytes: SPOOL_MAGIC.len() as u64,
            events: Vec::new(),
            last_tag: [0; 32],
        }
    }
}

pub(super) fn encode_frame(
    sequence: u64,
    key_id: &str,
    key: &[u8],
    previous_tag: [u8; 32],
    payload: &[u8],
) -> Result<(Vec<u8>, [u8; 32]), AuthenticatedSiemSpoolError> {
    if payload.len() > MAX_RECORD_BYTES {
        return Err(AuthenticatedSiemSpoolError::RecordTooLarge);
    }
    let tag = sign_frame(sequence, key_id, key, &previous_tag, payload)?;
    let prefix = format!(
        "{:08x}:{sequence:016x}:{:02x}:{key_id}:{}:{}:",
        payload.len(),
        key_id.len(),
        hex::encode(previous_tag),
        hex::encode(tag)
    );
    let capacity = prefix
        .len()
        .checked_add(payload.len())
        .and_then(|length| length.checked_add(1))
        .ok_or(AuthenticatedSiemSpoolError::RecordTooLarge)?;
    let mut frame = Vec::with_capacity(capacity);
    frame.extend_from_slice(prefix.as_bytes());
    frame.extend_from_slice(payload);
    frame.push(b'\n');
    Ok((frame, tag))
}

pub(super) fn read_and_verify(
    file: &mut File,
    max_bytes: u64,
    keys: &SiemKeyRing,
) -> Result<DecodedSpool, AuthenticatedSiemSpoolError> {
    let length = file
        .metadata()
        .map_err(|error| io_failure("metadata", &error))?
        .len();
    if length > max_bytes {
        return Err(AuthenticatedSiemSpoolError::CapacityExceeded);
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| io_failure("seek", &error))?;
    let mut bytes = Vec::with_capacity(length as usize);
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_failure("read", &error))?;
    file.seek(SeekFrom::End(0))
        .map_err(|error| io_failure("seek", &error))?;
    if bytes.len() as u64 != length {
        return Err(AuthenticatedSiemSpoolError::ExternalModification);
    }
    if !bytes.starts_with(SPOOL_MAGIC) {
        return Err(corrupt(0, "unsupported or missing header"));
    }

    let mut cursor = SPOOL_MAGIC.len();
    let mut events = Vec::new();
    let mut last_tag = [0_u8; 32];
    while cursor < bytes.len() {
        if events.len() >= super::MAX_SIEM_SPOOL_RECORDS {
            return Err(AuthenticatedSiemSpoolError::RecordCapacityExceeded);
        }
        let relative_end = bytes[cursor..]
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| corrupt(events.len() + 1, "truncated frame"))?;
        let end = cursor
            .checked_add(relative_end)
            .ok_or_else(|| corrupt(events.len() + 1, "invalid frame offset"))?;
        let record = events.len() + 1;
        let expected_sequence = u64::try_from(record)
            .map_err(|_| corrupt(record, "sequence exceeds platform limit"))?;
        let (event, tag) = decode_frame(
            &bytes[cursor..end],
            record,
            expected_sequence,
            last_tag,
            keys,
        )?;
        events.push(event);
        last_tag = tag;
        cursor = end
            .checked_add(1)
            .ok_or_else(|| corrupt(record, "invalid frame offset"))?;
    }
    Ok(DecodedSpool {
        bytes: length,
        events,
        last_tag,
    })
}

fn decode_frame(
    frame: &[u8],
    record: usize,
    expected_sequence: u64,
    expected_previous_tag: [u8; 32],
    keys: &SiemKeyRing,
) -> Result<(LiveSecurityEvent, [u8; 32]), AuthenticatedSiemSpoolError> {
    let minimum =
        LENGTH_HEX_BYTES + SEQUENCE_HEX_BYTES + KEY_LENGTH_HEX_BYTES + (TAG_HEX_BYTES * 2) + 6;
    if frame.len() < minimum {
        return Err(corrupt(record, "invalid frame header"));
    }
    let payload_length = parse_hex_usize(field(frame, 0, LENGTH_HEX_BYTES, record)?, record)?;
    if payload_length > MAX_RECORD_BYTES {
        return Err(corrupt(record, "record length exceeds limit"));
    }
    let sequence_start = LENGTH_HEX_BYTES + 1;
    let sequence_bytes = field(frame, sequence_start, SEQUENCE_HEX_BYTES, record)?;
    let sequence = parse_hex_u64(sequence_bytes, record)?;
    if sequence != expected_sequence || format!("{sequence:016x}").as_bytes() != sequence_bytes {
        return Err(corrupt(record, "non-canonical or discontinuous sequence"));
    }
    let key_length_start = sequence_start + SEQUENCE_HEX_BYTES + 1;
    let key_length_bytes = field(frame, key_length_start, KEY_LENGTH_HEX_BYTES, record)?;
    let key_length = parse_hex_usize(key_length_bytes, record)?;
    if key_length == 0
        || key_length > 32
        || format!("{key_length:02x}").as_bytes() != key_length_bytes
    {
        return Err(corrupt(record, "invalid key identifier length"));
    }
    let key_start = key_length_start + KEY_LENGTH_HEX_BYTES + 1;
    let key_bytes = field(frame, key_start, key_length, record)?;
    let key_id =
        std::str::from_utf8(key_bytes).map_err(|_| corrupt(record, "invalid key identifier"))?;
    if !key_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(corrupt(record, "invalid key identifier"));
    }

    let previous_start = key_start + key_length + 1;
    let previous_bytes = field(frame, previous_start, TAG_HEX_BYTES, record)?;
    let previous_tag = decode_tag(previous_bytes, record, "invalid predecessor tag")?;
    if previous_tag.ct_eq(&expected_previous_tag).unwrap_u8() != 1 {
        return Err(corrupt(record, "predecessor chain mismatch"));
    }
    let tag_start = previous_start + TAG_HEX_BYTES + 1;
    let tag_bytes = field(frame, tag_start, TAG_HEX_BYTES, record)?;
    let recorded_tag = decode_tag(tag_bytes, record, "invalid authentication tag")?;
    let payload_start = tag_start + TAG_HEX_BYTES + 1;
    let payload = frame
        .get(payload_start..)
        .ok_or_else(|| corrupt(record, "missing payload"))?;
    if payload.len() != payload_length
        || format!("{payload_length:08x}").as_bytes() != &frame[..LENGTH_HEX_BYTES]
    {
        return Err(corrupt(record, "record length mismatch"));
    }

    let key = keys
        .get(key_id)
        .ok_or(AuthenticatedSiemSpoolError::UnknownKey { record })?;
    let expected_tag = sign_frame(sequence, key_id, key, &previous_tag, payload)?;
    if expected_tag.ct_eq(&recorded_tag).unwrap_u8() != 1 {
        return Err(AuthenticatedSiemSpoolError::AuthenticationFailed { record });
    }

    let event: LiveSecurityEvent =
        serde_json::from_slice(payload).map_err(|_| corrupt(record, "invalid event JSON"))?;
    let mut normalized = event.clone();
    normalized.verified_hmac = false;
    normalized = normalized.normalized();
    if event.schema_version != SECURITY_EVENT_SCHEMA_VERSION || event != normalized {
        return Err(corrupt(
            record,
            "event violates the unsigned local v1 contract",
        ));
    }
    let mut verified = event;
    verified.verified_hmac = true;
    Ok((verified, recorded_tag))
}

fn field(
    frame: &[u8],
    start: usize,
    length: usize,
    record: usize,
) -> Result<&[u8], AuthenticatedSiemSpoolError> {
    let end = start
        .checked_add(length)
        .ok_or_else(|| corrupt(record, "invalid frame offset"))?;
    if frame.get(end) != Some(&b':') {
        return Err(corrupt(record, "invalid frame delimiter"));
    }
    frame
        .get(start..end)
        .ok_or_else(|| corrupt(record, "truncated frame header"))
}

fn parse_hex_usize(bytes: &[u8], record: usize) -> Result<usize, AuthenticatedSiemSpoolError> {
    let value = std::str::from_utf8(bytes).map_err(|_| corrupt(record, "invalid hexadecimal"))?;
    usize::from_str_radix(value, 16).map_err(|_| corrupt(record, "invalid hexadecimal"))
}

fn parse_hex_u64(bytes: &[u8], record: usize) -> Result<u64, AuthenticatedSiemSpoolError> {
    let value = std::str::from_utf8(bytes).map_err(|_| corrupt(record, "invalid hexadecimal"))?;
    u64::from_str_radix(value, 16).map_err(|_| corrupt(record, "invalid hexadecimal"))
}

fn decode_tag(
    bytes: &[u8],
    record: usize,
    reason: &'static str,
) -> Result<[u8; 32], AuthenticatedSiemSpoolError> {
    let text = std::str::from_utf8(bytes).map_err(|_| corrupt(record, reason))?;
    let decoded = hex::decode(text).map_err(|_| corrupt(record, reason))?;
    let tag: [u8; 32] = decoded.try_into().map_err(|_| corrupt(record, reason))?;
    if hex::encode(tag).as_bytes() != bytes {
        return Err(corrupt(record, reason));
    }
    Ok(tag)
}

fn sign_frame(
    sequence: u64,
    key_id: &str,
    key: &[u8],
    previous_tag: &[u8; 32],
    payload: &[u8],
) -> Result<[u8; 32], AuthenticatedSiemSpoolError> {
    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(key)
        .map_err(|_| AuthenticatedSiemSpoolError::InvalidKeyMaterial)?;
    mac.update(HMAC_DOMAIN);
    mac.update(&sequence.to_be_bytes());
    mac.update(&(key_id.len() as u64).to_be_bytes());
    mac.update(key_id.as_bytes());
    mac.update(previous_tag);
    mac.update(&(payload.len() as u64).to_be_bytes());
    mac.update(payload);
    Ok(mac.finalize().into_bytes().into())
}

const fn corrupt(record: usize, reason: &'static str) -> AuthenticatedSiemSpoolError {
    AuthenticatedSiemSpoolError::CorruptRecord { record, reason }
}

fn io_failure(operation: &'static str, error: &std::io::Error) -> AuthenticatedSiemSpoolError {
    AuthenticatedSiemSpoolError::Io {
        operation,
        kind: error.kind(),
    }
}
