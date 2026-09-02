//! Strict W3C trace-context propagation without arbitrary baggage forwarding.

use crate::{MessageHeaders, MessagingError, Result};
use std::collections::BTreeSet;
use std::fmt;

const TRACEPARENT_BYTES: usize = 55;
const MAX_TRACESTATE_BYTES: usize = 512;
const MAX_TRACESTATE_ENTRIES: usize = 32;

/// Validated W3C `traceparent` and optional strict-subset `tracestate` values.
///
/// Baggage is deliberately excluded because arbitrary application metadata can
/// contain credentials or personal data. The host still owns sampling,
/// exporter configuration, retention, and tenant-aware correlation policy.
#[derive(Clone, Eq, PartialEq)]
pub struct TraceContext {
    traceparent: String,
    tracestate: Option<String>,
}

impl TraceContext {
    /// Validates version-00 W3C trace context and a conservative tracestate subset.
    pub fn try_new(traceparent: impl Into<String>) -> Result<Self> {
        let traceparent = traceparent.into();
        validate_traceparent(&traceparent)?;
        Ok(Self {
            traceparent,
            tracestate: None,
        })
    }

    /// Validates version-00 W3C trace context with a conservative tracestate subset.
    pub fn try_with_state(
        traceparent: impl Into<String>,
        tracestate: impl Into<String>,
    ) -> Result<Self> {
        let mut context = Self::try_new(traceparent)?;
        let tracestate = tracestate.into();
        validate_tracestate(&tracestate)?;
        context.tracestate = Some(tracestate);
        Ok(context)
    }

    /// Returns the validated `traceparent` value.
    pub fn traceparent(&self) -> &str {
        &self.traceparent
    }

    /// Returns the optional validated `tracestate` value.
    pub fn tracestate(&self) -> Option<&str> {
        self.tracestate.as_deref()
    }

    pub(crate) fn insert_into(&self, headers: &mut MessageHeaders) -> Result<()> {
        headers.try_insert("traceparent", &self.traceparent)?;
        if let Some(tracestate) = &self.tracestate {
            headers.try_insert("tracestate", tracestate)?;
        }
        Ok(())
    }

    pub(crate) fn from_headers(headers: &MessageHeaders) -> Result<Option<Self>> {
        match (headers.get("traceparent"), headers.get("tracestate")) {
            (None, None) => Ok(None),
            (None, Some(_)) => Err(invalid("tracestate requires traceparent")),
            (Some(traceparent), None) => Self::try_new(traceparent).map(Some),
            (Some(traceparent), Some(tracestate)) => {
                Self::try_with_state(traceparent, tracestate).map(Some)
            }
        }
    }
}

impl fmt::Debug for TraceContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TraceContext")
            .field("traceparent", &"[REDACTED]")
            .field("has_tracestate", &self.tracestate.is_some())
            .finish()
    }
}

fn validate_traceparent(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let separators =
        bytes.get(2) == Some(&b'-') && bytes.get(35) == Some(&b'-') && bytes.get(52) == Some(&b'-');
    if bytes.len() != TRACEPARENT_BYTES
        || !separators
        || bytes.get(..2) != Some(b"00".as_slice())
        || !hex_lower(bytes.get(3..35))
        || !hex_lower(bytes.get(36..52))
        || !hex_lower(bytes.get(53..55))
        || all_zero(bytes.get(3..35))
        || all_zero(bytes.get(36..52))
    {
        return Err(invalid("traceparent must be canonical W3C version 00"));
    }
    Ok(())
}

fn validate_tracestate(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_TRACESTATE_BYTES
        || value.trim() != value
        || value.split(',').count() > MAX_TRACESTATE_ENTRIES
    {
        return Err(invalid("tracestate is outside the supported bounds"));
    }
    let mut keys = BTreeSet::new();
    for member in value.split(',') {
        let Some((key, state)) = member.split_once('=') else {
            return Err(invalid("tracestate member is malformed"));
        };
        if !valid_tracestate_key(key)
            || !keys.insert(key)
            || state.is_empty()
            || state.len() > 256
            || !state
                .bytes()
                .all(|byte| matches!(byte, 0x21..=0x2b | 0x2d..=0x3c | 0x3e..=0x7e))
        {
            return Err(invalid("tracestate member is malformed"));
        }
    }
    Ok(())
}

fn valid_tracestate_key(key: &str) -> bool {
    let Some((tenant, system)) = key.split_once('@') else {
        return key.len() <= 256
            && key.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            && key.bytes().all(valid_key_byte);
    };
    !tenant.is_empty()
        && tenant.len() <= 241
        && tenant
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && tenant.bytes().all(valid_key_byte)
        && !system.is_empty()
        && system.len() <= 14
        && system
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_lowercase)
        && system.bytes().all(valid_key_byte)
}

fn valid_key_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'*' | b'/')
}

fn hex_lower(bytes: Option<&[u8]>) -> bool {
    bytes.is_some_and(|bytes| {
        bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}

fn all_zero(bytes: Option<&[u8]>) -> bool {
    bytes.is_some_and(|bytes| bytes.iter().all(|byte| *byte == b'0'))
}

const fn invalid(reason: &'static str) -> MessagingError {
    MessagingError::Invalid {
        field: "trace context",
        reason,
    }
}

#[cfg(test)]
mod tests;
