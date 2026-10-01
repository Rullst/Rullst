//! JSON encoding of Hrana values.

use base64::{
    Engine as _, alphabet,
    engine::{
        DecodePaddingMode,
        general_purpose::{GeneralPurpose, GeneralPurposeConfig, STANDARD as BASE64},
    },
};
use serde::{Deserialize, Serialize};

use super::{PolyglotError, TursoValue};

/// Decoder for Hrana `blob` cells. The Hrana 3 specification
/// (<https://github.com/tursodatabase/libsql/blob/main/docs/HRANA_3_SPEC.md>,
/// "Values") only says a JSON blob is "base64-encoded"; libSQL server
/// (`libsql-hrana/src/proto.rs`, `bytes_as_base64`) encodes it with standard
/// base64 *without* padding and trims `=` before decoding. Requests keep
/// canonical padding, which that decoder accepts, while responses decode with
/// or without it.
const BLOB_DECODER: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum WireValue {
    Null,
    Integer { value: String },
    Float { value: f64 },
    Text { value: String },
    Blob { base64: String },
}

impl From<TursoValue> for WireValue {
    fn from(value: TursoValue) -> Self {
        match value {
            TursoValue::Null => Self::Null,
            TursoValue::Integer(value) => Self::Integer {
                value: value.to_string(),
            },
            TursoValue::Real(value) => Self::Float { value },
            TursoValue::Text(value) => Self::Text { value },
            TursoValue::Blob(value) => Self::Blob {
                base64: BASE64.encode(value),
            },
        }
    }
}

impl TryFrom<WireValue> for TursoValue {
    type Error = PolyglotError;

    fn try_from(value: WireValue) -> Result<Self, Self::Error> {
        match value {
            WireValue::Null => Ok(Self::Null),
            WireValue::Integer { value } => value
                .parse()
                .map(Self::Integer)
                .map_err(PolyglotError::serialization),
            WireValue::Float { value } => Ok(Self::Real(value)),
            WireValue::Text { value } => Ok(Self::Text(value)),
            WireValue::Blob { base64 } => BLOB_DECODER
                .decode(base64)
                .map(Self::Blob)
                .map_err(PolyglotError::serialization),
        }
    }
}
