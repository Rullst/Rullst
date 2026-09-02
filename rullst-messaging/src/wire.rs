//! Canonical bounded wire representation for future broker adapters.

use crate::model::StoredEnvelopeParts;
use crate::validation::{
    MAX_CONTENT_TYPE_BYTES, MAX_EVENT_KIND_BYTES, MAX_HEADER_COUNT, MAX_HEADER_TOTAL_BYTES,
    MAX_NAMESPACE_BYTES, MAX_TOPIC_BYTES,
};
use crate::{
    BrokerConfig, ContentType, EventKind, MessageEnvelope, MessageHeaders, MessageId,
    MessagingError, Namespace, Result, TopicName,
};

const MAGIC_PREFIX: &[u8; 7] = b"RLMWIRE";
const VERSION: u8 = 1;
const MESSAGE_ID_BYTES: usize = 36;
const FIXED_BYTES: usize = MAGIC_PREFIX.len() + 1 + 8 + 2 + 2 + 2 + 2 + 2 + 2 + 4;
const MAX_WIRE_OVERHEAD: usize = FIXED_BYTES
    + MESSAGE_ID_BYTES
    + MAX_NAMESPACE_BYTES
    + MAX_TOPIC_BYTES
    + MAX_EVENT_KIND_BYTES
    + MAX_CONTENT_TYPE_BYTES
    + MAX_HEADER_TOTAL_BYTES
    + MAX_HEADER_COUNT * 4;

/// Version-1 canonical binary codec for broker-neutral message envelopes.
///
/// This codec is a local interoperability contract for future adapters, not a
/// claim that any remote broker protocol is currently implemented. Decoding is
/// bound by the supplied broker configuration, requires its exact namespace,
/// rejects unknown versions/trailing bytes/non-canonical header order, and
/// revalidates every public value before allocating the payload.
#[derive(Clone, Copy, Debug, Default)]
pub struct WireEnvelopeCodec;

impl WireEnvelopeCodec {
    /// Encodes one already validated envelope in canonical version-1 form.
    pub fn encode(envelope: &MessageEnvelope, config: &BrokerConfig) -> Result<Vec<u8>> {
        validate_envelope(envelope, config)?;
        let maximum = maximum_wire_bytes(config)?;
        let estimated = FIXED_BYTES
            .checked_add(envelope.id().as_str().len())
            .and_then(|size| size.checked_add(envelope.namespace().as_str().len()))
            .and_then(|size| size.checked_add(envelope.topic().as_str().len()))
            .and_then(|size| size.checked_add(envelope.event_kind().as_str().len()))
            .and_then(|size| size.checked_add(envelope.content_type().as_str().len()))
            .and_then(|size| size.checked_add(envelope.payload().len()))
            .and_then(|size| {
                envelope
                    .headers()
                    .iter()
                    .try_fold(size, |total, (name, value)| {
                        total
                            .checked_add(4)
                            .and_then(|next| next.checked_add(name.len()))
                            .and_then(|next| next.checked_add(value.len()))
                    })
            })
            .ok_or(MessagingError::InvalidWireEnvelope)?;
        if estimated > maximum {
            return Err(MessagingError::InvalidWireEnvelope);
        }
        let mut frame = Vec::with_capacity(estimated);
        frame.extend_from_slice(MAGIC_PREFIX);
        frame.push(VERSION);
        put_string(&mut frame, envelope.id().as_str())?;
        put_string(&mut frame, envelope.namespace().as_str())?;
        put_string(&mut frame, envelope.topic().as_str())?;
        put_string(&mut frame, envelope.event_kind().as_str())?;
        put_string(&mut frame, envelope.content_type().as_str())?;
        frame.extend_from_slice(&envelope.published_at_ms().to_be_bytes());
        let header_count = u16::try_from(envelope.headers().len())
            .map_err(|_| MessagingError::InvalidWireEnvelope)?;
        frame.extend_from_slice(&header_count.to_be_bytes());
        for (name, value) in envelope.headers().iter() {
            put_string(&mut frame, name)?;
            put_string(&mut frame, value)?;
        }
        let payload_length = u32::try_from(envelope.payload().len())
            .map_err(|_| MessagingError::InvalidWireEnvelope)?;
        frame.extend_from_slice(&payload_length.to_be_bytes());
        frame.extend_from_slice(envelope.payload());
        Ok(frame)
    }

    /// Decodes, bounds, and revalidates one canonical version-1 frame.
    pub fn decode(frame: &[u8], config: &BrokerConfig) -> Result<MessageEnvelope> {
        if frame.len() > maximum_wire_bytes(config)? {
            return Err(MessagingError::InvalidWireEnvelope);
        }
        let mut reader = Reader::new(frame);
        if reader.take(MAGIC_PREFIX.len())? != MAGIC_PREFIX {
            return Err(MessagingError::InvalidWireEnvelope);
        }
        if reader.byte()? != VERSION {
            return Err(MessagingError::UnsupportedWireVersion);
        }
        let id = MessageId::from_stored(reader.string()?)
            .map_err(|_| MessagingError::InvalidWireEnvelope)?;
        let namespace = Namespace::try_new(reader.string()?)
            .map_err(|_| MessagingError::InvalidWireEnvelope)?;
        if &namespace != config.namespace() {
            return Err(MessagingError::InvalidWireEnvelope);
        }
        let topic = TopicName::try_new(reader.string()?)
            .map_err(|_| MessagingError::InvalidWireEnvelope)?;
        let event_kind = EventKind::try_new(reader.string()?)
            .map_err(|_| MessagingError::InvalidWireEnvelope)?;
        let content_type = ContentType::try_new(reader.string()?)
            .map_err(|_| MessagingError::InvalidWireEnvelope)?;
        let published_at_ms = reader.i64()?;
        if published_at_ms < 0 {
            return Err(MessagingError::InvalidWireEnvelope);
        }
        let header_count = usize::from(reader.u16()?);
        if header_count > MAX_HEADER_COUNT {
            return Err(MessagingError::InvalidWireEnvelope);
        }
        let mut headers = MessageHeaders::new();
        let mut previous_name: Option<String> = None;
        for _ in 0..header_count {
            let name = reader.string()?;
            if previous_name
                .as_deref()
                .is_some_and(|previous| previous >= name.as_str())
            {
                return Err(MessagingError::InvalidWireEnvelope);
            }
            let value = reader.string()?;
            headers
                .try_insert(&name, value)
                .map_err(|_| MessagingError::InvalidWireEnvelope)?;
            previous_name = Some(name);
        }
        let payload_length =
            usize::try_from(reader.u32()?).map_err(|_| MessagingError::InvalidWireEnvelope)?;
        if payload_length > config.max_payload_bytes() {
            return Err(MessagingError::InvalidWireEnvelope);
        }
        let payload = reader.take(payload_length)?.to_vec();
        if !reader.is_finished() {
            return Err(MessagingError::InvalidWireEnvelope);
        }
        let envelope = MessageEnvelope::from_stored(StoredEnvelopeParts {
            id,
            namespace,
            topic,
            event_kind,
            content_type,
            headers,
            payload,
            published_at_ms,
        });
        if Self::encode(&envelope, config)?.as_slice() != frame {
            return Err(MessagingError::InvalidWireEnvelope);
        }
        Ok(envelope)
    }
}

fn validate_envelope(envelope: &MessageEnvelope, config: &BrokerConfig) -> Result<()> {
    if envelope.schema() != MessageEnvelope::SCHEMA
        || envelope.namespace() != config.namespace()
        || envelope.published_at_ms() < 0
        || envelope.payload().len() > config.max_payload_bytes()
    {
        return Err(MessagingError::InvalidWireEnvelope);
    }
    Ok(())
}

fn maximum_wire_bytes(config: &BrokerConfig) -> Result<usize> {
    config
        .max_payload_bytes()
        .checked_add(MAX_WIRE_OVERHEAD)
        .ok_or(MessagingError::InvalidWireEnvelope)
}

fn put_string(frame: &mut Vec<u8>, value: &str) -> Result<()> {
    let length = u16::try_from(value.len()).map_err(|_| MessagingError::InvalidWireEnvelope)?;
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(value.as_bytes());
    Ok(())
}

struct Reader<'a> {
    frame: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    const fn new(frame: &'a [u8]) -> Self {
        Self { frame, cursor: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(MessagingError::InvalidWireEnvelope)?;
        let bytes = self
            .frame
            .get(self.cursor..end)
            .ok_or(MessagingError::InvalidWireEnvelope)?;
        self.cursor = end;
        Ok(bytes)
    }

    fn byte(&mut self) -> Result<u8> {
        self.take(1)?
            .first()
            .copied()
            .ok_or(MessagingError::InvalidWireEnvelope)
    }

    fn u16(&mut self) -> Result<u16> {
        let bytes: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| MessagingError::InvalidWireEnvelope)?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| MessagingError::InvalidWireEnvelope)?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn i64(&mut self) -> Result<i64> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| MessagingError::InvalidWireEnvelope)?;
        Ok(i64::from_be_bytes(bytes))
    }

    fn string(&mut self) -> Result<String> {
        let length = usize::from(self.u16()?);
        std::str::from_utf8(self.take(length)?)
            .map(str::to_string)
            .map_err(|_| MessagingError::InvalidWireEnvelope)
    }

    fn is_finished(&self) -> bool {
        self.cursor == self.frame.len()
    }
}

#[cfg(test)]
mod tests;
