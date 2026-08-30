use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::super::PolyglotError;

const MAX_COLLECTION_NAME_BYTES: usize = 64;
const MAX_VECTOR_DIMENSIONS: usize = 65_536;
const MAX_QUERY_LIMIT: u16 = 1_000;
const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;

/// A deliberately restricted Qdrant collection name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorCollectionName(String);

impl VectorCollectionName {
    /// Validates an ASCII collection name before it can enter a URL path.
    pub fn new(value: impl Into<String>) -> Result<Self, PolyglotError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_COLLECTION_NAME_BYTES
            && value
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
        if !valid {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Qdrant collection",
                reason: "use 1-64 ASCII letters, digits, underscores, or hyphens and start with a letter or digit",
            });
        }
        Ok(Self(value))
    }

    /// Returns the validated collection name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated dense-vector dimensionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VectorDimensions(u32);

impl VectorDimensions {
    /// Creates a dimension bound accepted by this adapter.
    pub fn new(value: usize) -> Result<Self, PolyglotError> {
        if !(1..=MAX_VECTOR_DIMENSIONS).contains(&value) {
            return Err(invalid_vector("dimensions must be between 1 and 65,536"));
        }
        Ok(Self(value as u32))
    }

    /// Returns the dimension count.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A validated maximum number of nearest-neighbor matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VectorQueryLimit(u16);

impl VectorQueryLimit {
    /// Creates a query limit from 1 through 1,000.
    pub fn new(value: u16) -> Result<Self, PolyglotError> {
        if !(1..=MAX_QUERY_LIMIT).contains(&value) {
            return Err(PolyglotError::InvalidConfiguration {
                backend: "Qdrant",
                reason: "query limit must be between 1 and 1,000",
            });
        }
        Ok(Self(value))
    }

    /// Returns the maximum number of matches.
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// A dense vector and bounded JSON payload stored under a numeric point ID.
#[derive(Debug, Clone, Serialize)]
pub struct VectorPoint {
    id: u64,
    vector: Vec<f32>,
    payload: Map<String, Value>,
}

impl VectorPoint {
    /// Creates and validates one dense-vector point.
    pub fn new(
        id: u64,
        vector: Vec<f32>,
        payload: Map<String, Value>,
    ) -> Result<Self, PolyglotError> {
        validate_vector(&vector, None)?;
        validate_payload(&payload)?;
        Ok(Self {
            id,
            vector,
            payload,
        })
    }

    /// Returns the stable numeric point identifier.
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the validated dense vector.
    pub fn vector(&self) -> &[f32] {
        &self.vector
    }

    /// Returns the bounded JSON payload.
    pub fn payload(&self) -> &Map<String, Value> {
        &self.payload
    }
}

/// One bounded nearest-neighbor result.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct VectorMatch {
    id: u64,
    score: f32,
    #[serde(default)]
    payload: Map<String, Value>,
}

impl VectorMatch {
    pub(super) fn new(id: u64, score: f32, payload: Map<String, Value>) -> Self {
        Self { id, score, payload }
    }

    /// Returns the point identifier.
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns Qdrant-compatible cosine similarity, where larger is closer.
    pub const fn score(&self) -> f32 {
        self.score
    }

    /// Returns the stored JSON payload.
    pub fn payload(&self) -> &Map<String, Value> {
        &self.payload
    }
}

pub(super) fn validate_vector(
    vector: &[f32],
    expected: Option<VectorDimensions>,
) -> Result<(), PolyglotError> {
    if vector.is_empty() || vector.len() > MAX_VECTOR_DIMENSIONS {
        return Err(invalid_vector(
            "vectors must contain between 1 and 65,536 values",
        ));
    }
    if expected.is_some_and(|dimensions| vector.len() != dimensions.get() as usize) {
        return Err(invalid_vector(
            "vector dimensions do not match the collection",
        ));
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(invalid_vector("vector values must be finite"));
    }
    let squared_norm = vector
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>();
    if squared_norm == 0.0 || !squared_norm.is_finite() {
        return Err(invalid_vector(
            "cosine vectors must have a finite non-zero norm",
        ));
    }
    Ok(())
}

pub(super) fn validate_payload(payload: &Map<String, Value>) -> Result<(), PolyglotError> {
    let bytes = serde_json::to_vec(payload).map_err(PolyglotError::serialization)?;
    if bytes.len() > MAX_PAYLOAD_BYTES {
        return Err(PolyglotError::InvalidConfiguration {
            backend: "Qdrant",
            reason: "point payload must not exceed 1 MiB",
        });
    }
    Ok(())
}

fn invalid_vector(reason: &'static str) -> PolyglotError {
    PolyglotError::InvalidConfiguration {
        backend: "Qdrant",
        reason,
    }
}
