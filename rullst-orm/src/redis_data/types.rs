use crate::polyglot::PolyglotError;

const MAX_KEY_BYTES: usize = 128;
const MAX_FIELD_BYTES: usize = 128;
const MAX_MEMBER_BYTES: usize = 4 * 1024;
const MAX_VALUE_BYTES: usize = 1024 * 1024;
const MAX_SCAN_LIMIT: u16 = 1_000;

/// A validated logical key inside the store's immutable namespace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RedisDataKey(String);

impl RedisDataKey {
    /// Accepts a compact ASCII key rather than arbitrary Redis command text.
    pub fn new(value: impl Into<String>) -> Result<Self, PolyglotError> {
        let value = value.into();
        if !valid_identifier(&value, MAX_KEY_BYTES) {
            return Err(invalid(
                "data key must contain 1-128 ASCII letters, digits, dots, colons, dashes, or underscores",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the validated logical key.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated Redis hash field.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RedisField(String);

impl RedisField {
    /// Creates a bounded structural field name.
    pub fn new(value: impl Into<String>) -> Result<Self, PolyglotError> {
        let value = value.into();
        if !valid_identifier(&value, MAX_FIELD_BYTES) {
            return Err(invalid(
                "hash field must contain 1-128 ASCII letters, digits, dots, colons, dashes, or underscores",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the validated hash field.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A bounded UTF-8 Redis string value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisValue(String);

impl RedisValue {
    /// Creates a string value no larger than 1 MiB.
    pub fn new(value: impl Into<String>) -> Result<Self, PolyglotError> {
        let value = value.into();
        if value.len() > MAX_VALUE_BYTES {
            return Err(invalid("Redis value must not exceed 1 MiB"));
        }
        Ok(Self(value))
    }

    /// Returns the bounded value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the owned value.
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// A bounded UTF-8 set or sorted-set member.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RedisMember(String);

impl RedisMember {
    /// Creates a non-empty member no larger than 4 KiB.
    pub fn new(value: impl Into<String>) -> Result<Self, PolyglotError> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_MEMBER_BYTES || value.chars().any(char::is_control)
        {
            return Err(invalid(
                "member must contain 1-4,096 bytes without control characters",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the bounded member.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the owned member.
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// A bounded number of collection members to materialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedisScanLimit(u16);

impl RedisScanLimit {
    /// Creates a limit from 1 through 1,000.
    pub fn new(value: u16) -> Result<Self, PolyglotError> {
        if !(1..=MAX_SCAN_LIMIT).contains(&value) {
            return Err(invalid("scan limit must be between 1 and 1,000"));
        }
        Ok(Self(value))
    }

    /// Returns the materialization limit.
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Redis structure selected for explicit deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RedisStructure {
    /// A Redis hash.
    Hash,
    /// A Redis set.
    Set,
    /// A Redis sorted set.
    SortedSet,
}

/// One member and finite score returned from a sorted set.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredRedisMember {
    member: RedisMember,
    score: f64,
}

impl ScoredRedisMember {
    pub(super) fn new(member: RedisMember, score: f64) -> Result<Self, PolyglotError> {
        if !score.is_finite() {
            return Err(invalid("sorted-set score must be finite"));
        }
        Ok(Self { member, score })
    }

    /// Returns the bounded member.
    pub fn member(&self) -> &RedisMember {
        &self.member
    }

    /// Returns the finite sorted-set score.
    pub const fn score(&self) -> f64 {
        self.score
    }
}

pub(super) fn validate_score(score: f64) -> Result<(), PolyglotError> {
    if !score.is_finite() {
        return Err(invalid("sorted-set score must be finite"));
    }
    Ok(())
}

fn valid_identifier(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'-' | b'_'))
}

fn invalid(reason: &'static str) -> PolyglotError {
    PolyglotError::InvalidConfiguration {
        backend: "Redis",
        reason,
    }
}
