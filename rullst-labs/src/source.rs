use crate::{ContentHash, LabError, MAX_SOURCE_BYTES};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroizing;

/// Untrusted learner source. Debug is redacted; serialization deliberately
/// exposes plaintext only for the bounded transport/encrypted-storage boundary.
#[derive(Clone)]
pub struct RustSource(Zeroizing<String>);
impl RustSource {
    pub fn new(source: impl Into<String>) -> Result<Self, LabError> {
        let source = Zeroizing::new(source.into());
        if source.trim().is_empty()
            || source.len() > MAX_SOURCE_BYTES
            || source
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            return Err(LabError::InvalidInput);
        }
        Ok(Self(source))
    }
    /// For the encryption/isolated compiler boundary; never log this value.
    pub fn expose_source(&self) -> &str {
        &self.0
    }
    pub fn digest(&self) -> ContentHash {
        ContentHash::of(self.0.as_bytes())
    }
}
impl std::fmt::Debug for RustSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RustSource([REDACTED])")
    }
}
impl Serialize for RustSource {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for RustSource {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
