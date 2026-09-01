//! Bounded RFC 7252 CoAP request encoding without UDP/DTLS transport.

extern crate alloc;

use alloc::{string::String, vec, vec::Vec};
use core::fmt;

/// Conservative datagram ceiling intended to avoid IP fragmentation.
pub const MAX_COAP_DATAGRAM_BYTES: usize = 1152;

/// CoAP message type accepted by the request helper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoapMessageType {
    /// Requires acknowledgement/retransmission handling by the caller.
    Confirmable,
    /// Does not require an acknowledgement.
    NonConfirmable,
}

impl CoapMessageType {
    const fn bits(self) -> u8 {
        match self {
            Self::Confirmable => 0,
            Self::NonConfirmable => 1,
        }
    }
}

/// Request methods defined by the base CoAP specification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoapMethod {
    /// Retrieve a resource.
    Get,
    /// Submit a representation or processing request.
    Post,
    /// Create or replace a resource.
    Put,
    /// Delete a resource.
    Delete,
}

impl CoapMethod {
    const fn code(self) -> u8 {
        match self {
            Self::Get => 1,
            Self::Post => 2,
            Self::Put => 3,
            Self::Delete => 4,
        }
    }
}

/// Fail-closed CoAP request construction errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoapCodecError {
    /// RFC 7252 limits tokens to eight bytes.
    TokenTooLong,
    /// A simplified URI path segment is empty, contains `/`, or has controls.
    InvalidPathSegment,
    /// An option cannot be represented by the base option encoding.
    OptionTooLarge,
    /// The encoded request exceeds the bounded datagram ceiling.
    DatagramTooLarge,
}

impl fmt::Display for CoapCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TokenTooLong => "CoAP token exceeds eight bytes",
            Self::InvalidPathSegment => "CoAP URI path segment is invalid",
            Self::OptionTooLarge => "CoAP option cannot be represented",
            Self::DatagramTooLarge => "CoAP request exceeds the local datagram ceiling",
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CoapCodecError {}

/// Owned builder for one bounded CoAP request datagram.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoapRequest {
    message_type: CoapMessageType,
    method: CoapMethod,
    message_id: u16,
    token: Vec<u8>,
    path: Vec<String>,
    content_format: Option<u16>,
    payload: Vec<u8>,
}

impl CoapRequest {
    /// Creates a request header. The caller must generate and correlate tokens
    /// and message identifiers and must implement retransmission when needed.
    pub fn new(
        message_type: CoapMessageType,
        method: CoapMethod,
        message_id: u16,
        token: impl Into<Vec<u8>>,
    ) -> Result<Self, CoapCodecError> {
        let token = token.into();
        if token.len() > 8 {
            return Err(CoapCodecError::TokenTooLong);
        }
        Ok(Self {
            message_type,
            method,
            message_id,
            token,
            path: Vec::new(),
            content_format: None,
            payload: Vec::new(),
        })
    }

    /// Appends one decoded URI-Path segment. `/` separators are not accepted
    /// inside a segment; callers should append each segment separately.
    pub fn path_segment(mut self, segment: impl Into<String>) -> Result<Self, CoapCodecError> {
        let segment = segment.into();
        if segment.is_empty()
            || segment.contains('/')
            || segment.chars().any(|character| character.is_control())
        {
            return Err(CoapCodecError::InvalidPathSegment);
        }
        if segment.len() > MAX_COAP_DATAGRAM_BYTES {
            return Err(CoapCodecError::OptionTooLarge);
        }
        let existing_path_bytes = self.path.iter().try_fold(0usize, |total, current| {
            total.checked_add(3usize.saturating_add(current.len()))
        });
        let projected_path_bytes = existing_path_bytes
            .and_then(|total| total.checked_add(3usize.saturating_add(segment.len())))
            .and_then(|total| total.checked_add(4 + self.token.len()))
            .ok_or(CoapCodecError::DatagramTooLarge)?;
        if projected_path_bytes > MAX_COAP_DATAGRAM_BYTES {
            return Err(CoapCodecError::DatagramTooLarge);
        }
        self.path.push(segment);
        Ok(self)
    }

    /// Adds the numeric Content-Format option.
    #[must_use]
    pub const fn content_format(mut self, content_format: u16) -> Self {
        self.content_format = Some(content_format);
        self
    }

    /// Sets the request payload. A non-empty payload receives the `0xff` marker.
    pub fn payload(mut self, payload: impl Into<Vec<u8>>) -> Result<Self, CoapCodecError> {
        let payload = payload.into();
        if payload.len() > MAX_COAP_DATAGRAM_BYTES {
            return Err(CoapCodecError::DatagramTooLarge);
        }
        self.payload = payload;
        Ok(self)
    }

    /// Encodes the RFC 7252 header, token, ordered options, and payload.
    pub fn encode(&self) -> Result<Vec<u8>, CoapCodecError> {
        if self.payload.len() > MAX_COAP_DATAGRAM_BYTES {
            return Err(CoapCodecError::DatagramTooLarge);
        }
        let estimated_capacity = 4usize
            .saturating_add(self.token.len())
            .saturating_add(self.payload.len())
            .saturating_add(self.path.len().saturating_mul(4));
        let mut datagram = Vec::with_capacity(MAX_COAP_DATAGRAM_BYTES.min(estimated_capacity));
        datagram.push(0x40 | (self.message_type.bits() << 4) | self.token.len() as u8);
        datagram.push(self.method.code());
        datagram.extend_from_slice(&self.message_id.to_be_bytes());
        datagram.extend_from_slice(&self.token);

        let mut previous_option = 0u16;
        for segment in &self.path {
            append_option(&mut datagram, previous_option, 11, segment.as_bytes())?;
            previous_option = 11;
        }
        if let Some(content_format) = self.content_format {
            let value = minimal_u16(content_format);
            append_option(&mut datagram, previous_option, 12, value.as_slice())?;
        }
        if !self.payload.is_empty() {
            ensure_room(&datagram, 1usize.saturating_add(self.payload.len()))?;
            datagram.push(0xff);
            datagram.extend_from_slice(&self.payload);
        }
        if datagram.len() > MAX_COAP_DATAGRAM_BYTES {
            return Err(CoapCodecError::DatagramTooLarge);
        }
        Ok(datagram)
    }
}

fn append_option(
    datagram: &mut Vec<u8>,
    previous_number: u16,
    option_number: u16,
    value: &[u8],
) -> Result<(), CoapCodecError> {
    let delta = option_number
        .checked_sub(previous_number)
        .ok_or(CoapCodecError::OptionTooLarge)? as usize;
    let (delta_nibble, delta_extension) = option_component(delta)?;
    let (length_nibble, length_extension) = option_component(value.len())?;
    let encoded_length = 1usize
        .checked_add(delta_extension.len())
        .and_then(|length| length.checked_add(length_extension.len()))
        .and_then(|length| length.checked_add(value.len()))
        .ok_or(CoapCodecError::DatagramTooLarge)?;
    ensure_room(datagram, encoded_length)?;
    datagram.push((delta_nibble << 4) | length_nibble);
    datagram.extend_from_slice(&delta_extension);
    datagram.extend_from_slice(&length_extension);
    datagram.extend_from_slice(value);
    Ok(())
}

fn ensure_room(datagram: &[u8], additional: usize) -> Result<(), CoapCodecError> {
    let final_length = datagram
        .len()
        .checked_add(additional)
        .ok_or(CoapCodecError::DatagramTooLarge)?;
    if final_length > MAX_COAP_DATAGRAM_BYTES {
        return Err(CoapCodecError::DatagramTooLarge);
    }
    Ok(())
}

fn option_component(value: usize) -> Result<(u8, Vec<u8>), CoapCodecError> {
    match value {
        0..=12 => Ok((value as u8, Vec::new())),
        13..=268 => Ok((13, vec![(value - 13) as u8])),
        269..=65_804 => Ok((14, ((value - 269) as u16).to_be_bytes().to_vec())),
        _ => Err(CoapCodecError::OptionTooLarge),
    }
}

fn minimal_u16(value: u16) -> Vec<u8> {
    match value {
        0 => Vec::new(),
        1..=255 => vec![value as u8],
        _ => value.to_be_bytes().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_well_known_core_get_vector() {
        let datagram = CoapRequest::new(
            CoapMessageType::Confirmable,
            CoapMethod::Get,
            0x7d34,
            Vec::<u8>::new(),
        )
        .and_then(|request| request.path_segment(".well-known"))
        .and_then(|request| request.path_segment("core"))
        .and_then(|request| request.encode())
        .expect("bounded CoAP GET should encode");

        assert_eq!(
            datagram,
            [
                0x40, 0x01, 0x7d, 0x34, 0xbb, b'.', b'w', b'e', b'l', b'l', b'-', b'k', b'n', b'o',
                b'w', b'n', 0x04, b'c', b'o', b'r', b'e',
            ]
        );
    }

    #[test]
    fn encodes_token_content_format_and_payload_marker() {
        let datagram =
            CoapRequest::new(CoapMessageType::NonConfirmable, CoapMethod::Post, 7, [0xa1])
                .and_then(|request| request.path_segment("telemetry"))
                .and_then(|request| request.content_format(50).payload(b"{}".to_vec()))
                .and_then(|request| request.encode())
                .expect("bounded CoAP POST should encode");
        assert_eq!(
            datagram,
            [
                0x51, 0x02, 0x00, 0x07, 0xa1, 0xb9, b't', b'e', b'l', b'e', b'm', b'e', b't', b'r',
                b'y', 0x11, 0x32, 0xff, b'{', b'}',
            ]
        );
    }

    #[test]
    fn rejects_invalid_or_unbounded_input() {
        assert_eq!(
            CoapRequest::new(CoapMessageType::Confirmable, CoapMethod::Get, 1, [0; 9]),
            Err(CoapCodecError::TokenTooLong)
        );
        assert_eq!(
            CoapRequest::new(
                CoapMessageType::Confirmable,
                CoapMethod::Get,
                1,
                Vec::<u8>::new()
            )
            .and_then(|request| request.path_segment("bad/path")),
            Err(CoapCodecError::InvalidPathSegment)
        );
        assert_eq!(
            CoapRequest::new(
                CoapMessageType::Confirmable,
                CoapMethod::Put,
                1,
                Vec::<u8>::new()
            )
            .and_then(|request| request.payload(vec![0; MAX_COAP_DATAGRAM_BYTES]))
            .and_then(|request| request.encode()),
            Err(CoapCodecError::DatagramTooLarge)
        );
    }
}
