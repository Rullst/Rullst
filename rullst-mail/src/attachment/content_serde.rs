//! Serde form for attachment bytes.
//!
//! Serialization keeps the 12.1 wire format, one JSON number per byte, so that
//! 12.1 workers can read jobs queued by upgraded producers. Deserialization
//! accepts that integer array and also a standard base64 string, the compact
//! form 13.0 producers write, so a 12.x worker reads jobs from either.
//!
//! The integer array costs about one `serde_json::Value` (32 bytes) per
//! attachment byte while the queue converts the job to and from a JSON value;
//! the compact producer is left to 13.0 because it changes the wire format.

use base64::prelude::*;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt;

/// Caps a pre-allocation derived from an untrusted sequence length hint.
const MAX_PREALLOCATED_BYTES: usize = 1024 * 1024;

/// Writes the derived `Vec<u8>` form, a sequence of bytes, in every format.
pub(super) fn serialize<S: Serializer>(content: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(content)
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
    use serde::Serialize;
    use serde_json::json;

    /// The 12.1 `Attachment`, with derived serde for its bytes.
    #[derive(Serialize)]
    struct Attachment121<'a> {
        filename: &'a str,
        content: &'a Vec<u8>,
        mime_type: &'a str,
        cid: Option<&'a str>,
    }

    #[test]
    fn serialization_keeps_the_12_1_wire_format() {
        let attachment = Attachment::new("note.txt", vec![0, 1, 255], "text/plain").with_cid("c1");
        let legacy = Attachment121 {
            filename: &attachment.filename,
            content: &attachment.content,
            mime_type: &attachment.mime_type,
            cid: attachment.cid.as_deref(),
        };
        assert_eq!(
            serde_json::to_string(&attachment).unwrap(),
            serde_json::to_string(&legacy).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&attachment).unwrap()["content"],
            json!([0, 1, 255])
        );
        let message = Message::new()
            .to("member@example.com")
            .attach(attachment.clone());
        let value = serde_json::to_value(&message).unwrap();
        assert_eq!(value["attachments"][0]["content"], json!([0, 1, 255]));
    }

    #[test]
    fn legacy_arrays_and_base64_strings_both_decode() {
        let attachment = Attachment::new("note.txt", vec![1, 2, 3], "text/plain");
        let legacy =
            json!({"filename":"note.txt","content":[1,2,3],"mime_type":"text/plain","cid":null});
        let decoded: Attachment = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(decoded, attachment);
        let decoded: Attachment = serde_json::from_str(&legacy.to_string()).unwrap();
        assert_eq!(decoded, attachment);

        let mut compact = legacy.clone();
        compact["content"] = json!("AQID");
        let decoded: Attachment = serde_json::from_value(compact.clone()).unwrap();
        assert_eq!(decoded, attachment);
        let decoded: Attachment = serde_json::from_str(&compact.to_string()).unwrap();
        assert_eq!(decoded, attachment);

        for invalid in [json!("not base64!"), json!([1, 256]), json!(7)] {
            let mut value = legacy.clone();
            value["content"] = invalid;
            assert!(serde_json::from_value::<Attachment>(value).is_err());
        }
    }
}
