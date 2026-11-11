//! Bounded RFC 7252 CoAP request encoding without UDP/DTLS transport.

extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::fmt;

/// Conservative datagram ceiling intended to avoid IP fragmentation.
pub const MAX_COAP_DATAGRAM_BYTES: usize = 1152;
/// RFC 7252 section 5.10 limits one Uri-Path option value to 255 bytes.
const MAX_URI_PATH_SEGMENT_BYTES: usize = 255;

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
///
/// `#[non_exhaustive]` since 13.0.0-alpha.5: match it with a `_` arm.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoapCodecError {
    /// RFC 7252 limits tokens to eight bytes.
    TokenTooLong,
    /// A simplified URI path segment is empty, `.` or `..` (which RFC 7252
    /// forbids as Uri-Path values), contains `/`, or has controls.
    InvalidPathSegment,
    /// An option value exceeds its RFC 7252 length limit or cannot be
    /// represented by the base option encoding.
    OptionTooLarge,
    /// The encoded request exceeds the bounded datagram ceiling.
    DatagramTooLarge,
    /// The allocator could not provide a buffer for the request. The input is
    /// valid; it can succeed once memory is available. Added in
    /// 13.0.0-alpha.5; earlier releases aborted instead.
    AllocationFailed,
}

impl fmt::Display for CoapCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TokenTooLong => "CoAP token exceeds eight bytes",
            Self::InvalidPathSegment => "CoAP URI path segment is invalid",
            Self::OptionTooLarge => "CoAP option cannot be represented",
            Self::DatagramTooLarge => "CoAP request exceeds the local datagram ceiling",
            Self::AllocationFailed => "CoAP request buffer could not be allocated",
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
    /// inside a segment; callers should append each segment separately. The
    /// dot segments `.` and `..` are rejected because RFC 7252 section 5.10.1
    /// requires the URI to be resolved before it is split into options. A
    /// segment longer than the RFC 7252 limit of 255 bytes is rejected with
    /// [`CoapCodecError::OptionTooLarge`].
    pub fn path_segment(mut self, segment: impl Into<String>) -> Result<Self, CoapCodecError> {
        let segment = segment.into();
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('/')
            || segment.chars().any(|character| character.is_control())
        {
            return Err(CoapCodecError::InvalidPathSegment);
        }
        if segment.len() > MAX_URI_PATH_SEGMENT_BYTES {
            return Err(CoapCodecError::OptionTooLarge);
        }
        let existing_path_bytes = self.path.iter().try_fold(0usize, |total, current| {
            total.checked_add(uri_path_option_bytes(current.len())?)
        });
        let projected_path_bytes = existing_path_bytes
            .and_then(|total| total.checked_add(uri_path_option_bytes(segment.len())?))
            .and_then(|total| total.checked_add(4 + self.token.len()))
            .ok_or(CoapCodecError::DatagramTooLarge)?;
        if projected_path_bytes > MAX_COAP_DATAGRAM_BYTES {
            return Err(CoapCodecError::DatagramTooLarge);
        }
        self.path
            .try_reserve(1)
            .map_err(|_| CoapCodecError::AllocationFailed)?;
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
    ///
    /// The datagram is measured first and then written into a buffer of
    /// exactly its length, allocated fallibly: when the allocator refuses it,
    /// this returns [`CoapCodecError::AllocationFailed`] instead of aborting.
    pub fn encode(&self) -> Result<Vec<u8>, CoapCodecError> {
        if self.payload.len() > MAX_COAP_DATAGRAM_BYTES {
            return Err(CoapCodecError::DatagramTooLarge);
        }
        let mut length = ByteCount(0);
        self.write_to(&mut length)?;
        if length.0 > MAX_COAP_DATAGRAM_BYTES {
            return Err(CoapCodecError::DatagramTooLarge);
        }
        let mut datagram = Vec::new();
        datagram
            .try_reserve_exact(length.0)
            .map_err(|_| CoapCodecError::AllocationFailed)?;
        self.write_to(&mut datagram)?;
        Ok(datagram)
    }

    /// Writes the datagram layout to `out`, which counts or stores it.
    fn write_to(&self, out: &mut impl ByteSink) -> Result<(), CoapCodecError> {
        out.put(&[
            0x40 | (self.message_type.bits() << 4) | self.token.len() as u8,
            self.method.code(),
        ]);
        out.put(&self.message_id.to_be_bytes());
        out.put(&self.token);

        let mut previous_option = 0u16;
        for segment in &self.path {
            write_option(out, previous_option, 11, segment.as_bytes())?;
            previous_option = 11;
        }
        if let Some(content_format) = self.content_format {
            let (value, value_len) = minimal_u16(content_format);
            write_option(out, previous_option, 12, &value[..value_len])?;
        }
        if !self.payload.is_empty() {
            out.put(&[0xff]);
            out.put(&self.payload);
        }
        Ok(())
    }
}

/// Destination of [`CoapRequest::write_to`]: a byte counter for the sizing
/// pass, or the reserved datagram for the writing pass.
trait ByteSink {
    fn put(&mut self, bytes: &[u8]);
}

/// Counts the bytes of a layout without storing them.
struct ByteCount(usize);

impl ByteSink for ByteCount {
    fn put(&mut self, bytes: &[u8]) {
        self.0 = self.0.saturating_add(bytes.len());
    }
}

impl ByteSink for Vec<u8> {
    fn put(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
    }
}

fn write_option(
    out: &mut impl ByteSink,
    previous_number: u16,
    option_number: u16,
    value: &[u8],
) -> Result<(), CoapCodecError> {
    let delta = option_number
        .checked_sub(previous_number)
        .ok_or(CoapCodecError::OptionTooLarge)? as usize;
    let (delta_nibble, delta_extension) = option_component(delta)?;
    let (length_nibble, length_extension) = option_component(value.len())?;
    out.put(&[(delta_nibble << 4) | length_nibble]);
    out.put(delta_extension.as_slice());
    out.put(length_extension.as_slice());
    out.put(value);
    Ok(())
}

/// Exact encoded size of one Uri-Path option. Its delta (11 for the first
/// segment, 0 afterwards) always fits in the header nibble, so only the value
/// length can need extension bytes.
fn uri_path_option_bytes(length: usize) -> Option<usize> {
    let extension = match classify_option_component(length)? {
        CoapOptionComponent::Inline(_) => 0,
        CoapOptionComponent::ExtendedOne(_) => 1,
        CoapOptionComponent::ExtendedTwo(_) => 2,
    };
    1usize.checked_add(extension)?.checked_add(length)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CoapOptionComponent {
    Inline(u8),
    ExtendedOne(u8),
    ExtendedTwo([u8; 2]),
}

const fn classify_option_component(value: usize) -> Option<CoapOptionComponent> {
    match value {
        0..=12 => Some(CoapOptionComponent::Inline(value as u8)),
        13..=268 => Some(CoapOptionComponent::ExtendedOne((value - 13) as u8)),
        269..=65_804 => Some(CoapOptionComponent::ExtendedTwo(
            ((value - 269) as u16).to_be_bytes(),
        )),
        _ => None,
    }
}

/// The extension bytes of an option delta or length (zero to two).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OptionExtension {
    bytes: [u8; 2],
    len: usize,
}

impl OptionExtension {
    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

fn option_component(value: usize) -> Result<(u8, OptionExtension), CoapCodecError> {
    let extension = |bytes, len| OptionExtension { bytes, len };
    match classify_option_component(value) {
        Some(CoapOptionComponent::Inline(nibble)) => Ok((nibble, extension([0, 0], 0))),
        Some(CoapOptionComponent::ExtendedOne(byte)) => Ok((13, extension([byte, 0], 1))),
        Some(CoapOptionComponent::ExtendedTwo(bytes)) => Ok((14, extension(bytes, 2))),
        None => Err(CoapCodecError::OptionTooLarge),
    }
}

/// The shortest big-endian `uint` encoding of `value` (RFC 7252 section 3.2)
/// and its length: zero bytes for 0, one for values below 256.
fn minimal_u16(value: u16) -> ([u8; 2], usize) {
    match value {
        0 => ([0, 0], 0),
        1..=255 => ([value as u8, 0], 1),
        _ => (value.to_be_bytes(), 2),
    }
}

#[cfg(test)]
#[path = "coap_tests.rs"]
mod tests;

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_coap_option_component_layout() {
        let value: usize = kani::any();

        match classify_option_component(value) {
            Some(CoapOptionComponent::Inline(nibble)) => {
                assert!(value <= 12);
                assert_eq!(usize::from(nibble), value);
            }
            Some(CoapOptionComponent::ExtendedOne(extension)) => {
                assert!(value >= 13 && value <= 268);
                assert_eq!(usize::from(extension) + 13, value);
            }
            Some(CoapOptionComponent::ExtendedTwo(extension)) => {
                assert!(value >= 269 && value <= 65_804);
                assert_eq!(usize::from(u16::from_be_bytes(extension)) + 269, value);
            }
            None => assert!(value > 65_804),
        }
    }
}
