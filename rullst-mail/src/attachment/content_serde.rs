//! Compact serde form for attachment bytes.
//!
//! Human-readable formats such as the JSON mail queue store the bytes as one
//! standard base64 string. The derived `Vec<u8>` form wrote one JSON number per
//! byte, and `serde_json::to_value` then held one `Value` node per byte (about
//! 32 bytes each), so a 20 MiB attachment needed hundreds of MiB to enqueue.
//! Deserialization still accepts that legacy integer array. Binary formats keep
//! the previous byte-sequence encoding.

use base64::prelude::*;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt;

/// Caps a pre-allocation derived from an untrusted sequence length hint.
const MAX_PREALLOCATED_BYTES: usize = 1024 * 1024;

pub(super) fn serialize<S: Serializer>(content: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        serializer.serialize_str(&BASE64_STANDARD.encode(content))
    } else {
        serializer.collect_seq(content)
    }
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    if deserializer.is_human_readable() {
        deserializer.deserialize_any(ContentVisitor)
    } else {
        deserializer.deserialize_seq(ContentVisitor)
    }
}

struct ContentVisitor;

impl<'de> Visitor<'de> for ContentVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("base64 attachment content or an array of bytes")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Vec<u8>, E> {
        BASE64_STANDARD
            .decode(value)
            .map_err(|_| E::custom("attachment content is not valid base64"))
    }

    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Vec<u8>, E> {
        Ok(value.to_vec())
    }

    fn visit_byte_buf<E: de::Error>(self, value: Vec<u8>) -> Result<Vec<u8>, E> {
        Ok(value)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<u8>, A::Error> {
        let capacity = seq.size_hint().unwrap_or(0).min(MAX_PREALLOCATED_BYTES);
        let mut bytes = Vec::with_capacity(capacity);
        while let Some(byte) = seq.next_element::<u8>()? {
            bytes.push(byte);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Attachment, Message};
    use serde_json::{Value, json};

    #[test]
    fn json_content_is_base64_and_legacy_arrays_still_decode() {
        let attachment = Attachment::new("note.txt", vec![1, 2, 3], "text/plain");
        let value = serde_json::to_value(&attachment).unwrap();
        assert_eq!(value["content"], "AQID");
        let decoded: Attachment = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, attachment);

        let legacy =
            json!({"filename":"note.txt","content":[1,2,3],"mime_type":"text/plain","cid":null});
        let decoded: Attachment = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(decoded, attachment);
        let decoded: Attachment = serde_json::from_str(&legacy.to_string()).unwrap();
        assert_eq!(decoded, attachment);

        for invalid in [json!("not base64!"), json!([1, 256]), json!(7)] {
            let mut value = legacy.clone();
            value["content"] = invalid;
            assert!(serde_json::from_value::<Attachment>(value).is_err());
        }
    }

    #[test]
    fn queued_attachment_json_stays_close_to_the_byte_size() {
        let content = vec![0xA5; 1024 * 1024];
        let message = Message::new()
            .to("member@example.com")
            .attach(Attachment::new(
                "report.bin",
                content.clone(),
                "application/octet-stream",
            ));
        let value = serde_json::to_value(&message).unwrap();
        assert!(matches!(
            value["attachments"][0]["content"],
            Value::String(_)
        ));
        let encoded = serde_json::to_string(&value).unwrap();
        // Base64 is 4/3 of the input; the legacy array was about 4 bytes per byte.
        assert!(encoded.len() < content.len() * 3 / 2);
        let decoded: Message = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.attachments[0].content, content);
    }
}
