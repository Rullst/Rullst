use crate::LabError;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Reference(String);
impl Reference {
    pub fn new(value: impl Into<String>) -> Result<Self, LabError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 96
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(LabError::InvalidInput);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for Reference {
    type Error = LabError;
    fn try_from(value: String) -> Result<Self, LabError> {
        Self::new(value)
    }
}
impl From<Reference> for String {
    fn from(value: Reference) -> String {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContentHash(String);
impl ContentHash {
    pub fn new(value: impl Into<String>) -> Result<Self, LabError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(LabError::InvalidInput);
        }
        Ok(Self(value))
    }
    pub fn of(bytes: &[u8]) -> Self {
        Self(hex::encode(Sha256::digest(bytes)))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ContentHash {
    type Error = LabError;
    fn try_from(value: String) -> Result<Self, LabError> {
        Self::new(value)
    }
}
impl From<ContentHash> for String {
    fn from(value: ContentHash) -> String {
        value.0
    }
}

/// Resolve under server-authenticated tenant/course membership, not form claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub tenant: Reference,
    pub course: Reference,
}
impl Scope {
    pub fn new(tenant: impl Into<String>, course: impl Into<String>) -> Result<Self, LabError> {
        Ok(Self {
            tenant: Reference::new(tenant)?,
            course: Reference::new(course)?,
        })
    }
}
