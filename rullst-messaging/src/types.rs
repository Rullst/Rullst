//! Validated identifiers, configuration, and bounded message headers.

use crate::validation::{
    MAX_CONSUMER_BYTES, MAX_EVENT_KIND_BYTES, MAX_GROUP_BYTES, MAX_HEADER_COUNT,
    MAX_HEADER_TOTAL_BYTES, MAX_NAMESPACE_BYTES, MAX_TOPIC_BYTES, validate_content_type,
    validate_failure_code, validate_header_name, validate_header_value, validate_idempotency_key,
    validate_route_identifier,
};
use crate::{MessagingError, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

macro_rules! route_identifier {
    ($name:ident, $field:literal, $max:expr) => {
        #[doc = concat!("Validated broker ", $field, ".")]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Creates a validated ", $field, ".")]
            pub fn try_new(value: impl Into<String>) -> Result<Self> {
                let value = value.into();
                validate_route_identifier($field, &value, $max)?;
                Ok(Self(value))
            }

            #[doc = concat!("Returns the validated ", $field, ".")]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

route_identifier!(Namespace, "namespace", MAX_NAMESPACE_BYTES);
route_identifier!(TopicName, "topic", MAX_TOPIC_BYTES);
route_identifier!(ConsumerGroup, "consumer group", MAX_GROUP_BYTES);
route_identifier!(ConsumerName, "consumer name", MAX_CONSUMER_BYTES);
route_identifier!(EventKind, "event kind", MAX_EVENT_KIND_BYTES);

/// Caller-owned idempotency key, redacted from debug output.
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Creates a bounded idempotency key.
    pub fn try_new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_idempotency_key(&value)?;
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for IdempotencyKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("IdempotencyKey([REDACTED])")
    }
}

/// Validated parameter-free MIME type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ContentType(String);

impl ContentType {
    /// Creates a validated MIME type such as `application/json`.
    pub fn try_new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_content_type(&value)?;
        Ok(Self(value))
    }

    /// Returns the MIME type.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn binary() -> Self {
        Self("application/octet-stream".to_string())
    }
}

/// Stable low-cardinality processing failure code.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct FailureCode(String);

impl FailureCode {
    /// Creates a bounded code such as `handler.transient`.
    pub fn try_new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_failure_code(&value)?;
        Ok(Self(value))
    }

    /// Returns the validated code.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn max_attempts() -> Self {
        Self("delivery.max_attempts".to_string())
    }
}

/// Broker-assigned opaque message identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct MessageId(String);

impl MessageId {
    pub(crate) fn random() -> Self {
        Self(format!("msg_{}", uuid::Uuid::new_v4().simple()))
    }

    #[cfg(feature = "sqlite")]
    pub(crate) fn from_stored(value: String) -> Result<Self> {
        let valid = value.len() == 36
            && value.starts_with("msg_")
            && value[4..]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'));
        if !valid {
            return Err(MessagingError::CorruptStorage {
                context: "message identifier",
            });
        }
        Ok(Self(value))
    }

    /// Returns the opaque identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Single-use acknowledgement capability, redacted from debug output.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AckToken(String);

impl AckToken {
    pub(crate) fn random() -> Self {
        Self(format!("ack_{}", uuid::Uuid::new_v4().simple()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AckToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AckToken([REDACTED])")
    }
}

/// Bounded, deterministic message metadata.
#[derive(Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct MessageHeaders(BTreeMap<String, String>);

impl MessageHeaders {
    /// Creates an empty header collection.
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "sqlite")]
    pub(crate) fn from_stored(entries: BTreeMap<String, String>) -> Result<Self> {
        let mut headers = Self::new();
        for (name, value) in entries {
            headers
                .try_insert(name, value)
                .map_err(|_| MessagingError::CorruptStorage {
                    context: "message headers",
                })?;
        }
        Ok(headers)
    }

    /// Adds one unique header after validating name, value, count, and total bytes.
    pub fn try_insert(&mut self, name: impl Into<String>, value: impl Into<String>) -> Result<()> {
        let name = name.into();
        let value = value.into();
        validate_header_name(&name)?;
        validate_header_value(&value)?;
        if self.0.contains_key(&name) {
            return Err(MessagingError::Invalid {
                field: "message header name",
                reason: "duplicate names are not allowed",
            });
        }
        if self.0.len() >= MAX_HEADER_COUNT {
            return Err(MessagingError::CapacityExceeded {
                resource: "message headers",
                limit: MAX_HEADER_COUNT,
            });
        }
        let current = self
            .0
            .iter()
            .try_fold(0usize, |total, (key, item)| {
                total.checked_add(key.len())?.checked_add(item.len())
            })
            .ok_or(MessagingError::InternalState {
                context: "message header byte accounting",
            })?;
        let proposed = current
            .checked_add(name.len())
            .and_then(|total| total.checked_add(value.len()))
            .ok_or(MessagingError::InternalState {
                context: "message header byte accounting",
            })?;
        if proposed > MAX_HEADER_TOTAL_BYTES {
            return Err(MessagingError::CapacityExceeded {
                resource: "message header bytes",
                limit: MAX_HEADER_TOTAL_BYTES,
            });
        }
        self.0.insert(name, value);
        Ok(())
    }

    /// Returns a header without modifying the collection.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// Iterates over validated names and values in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// Returns the number of headers.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for MessageHeaders {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MessageHeaders")
            .field("names", &self.0.keys().collect::<Vec<_>>())
            .field("count", &self.0.len())
            .finish()
    }
}

/// Position used when a consumer group is first registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StartPosition {
    /// Deliver every message still retained by the broker.
    Earliest,
    /// Deliver only messages published after registration.
    Latest,
}

/// Hard bounds for a broker instance.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BrokerConfig {
    namespace: Namespace,
    max_retained_messages: usize,
    max_subscriptions: usize,
    max_attempts: u32,
    max_payload_bytes: usize,
}

impl BrokerConfig {
    /// Default retained-message bound.
    pub const DEFAULT_MAX_RETAINED_MESSAGES: usize = 10_000;
    /// Absolute retained-message configuration ceiling.
    pub const MAX_RETAINED_MESSAGES: usize = 100_000;
    /// Default subscription bound.
    pub const DEFAULT_MAX_SUBSCRIPTIONS: usize = 128;
    /// Absolute subscription configuration ceiling.
    pub const MAX_SUBSCRIPTIONS: usize = 1_024;
    /// Default delivery-attempt bound.
    pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;
    /// Absolute attempt configuration ceiling.
    pub const MAX_ATTEMPTS: u32 = 100;
    /// Default payload limit (one MiB).
    pub const DEFAULT_MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
    /// Absolute payload configuration ceiling (sixteen MiB).
    pub const MAX_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;

    /// Creates a bounded configuration with conservative defaults.
    pub fn try_new(namespace: impl Into<String>) -> Result<Self> {
        Ok(Self {
            namespace: Namespace::try_new(namespace)?,
            max_retained_messages: Self::DEFAULT_MAX_RETAINED_MESSAGES,
            max_subscriptions: Self::DEFAULT_MAX_SUBSCRIPTIONS,
            max_attempts: Self::DEFAULT_MAX_ATTEMPTS,
            max_payload_bytes: Self::DEFAULT_MAX_PAYLOAD_BYTES,
        })
    }

    /// Replaces all operational limits after validating their ceilings.
    pub fn with_limits(
        mut self,
        max_retained_messages: usize,
        max_subscriptions: usize,
        max_attempts: u32,
        max_payload_bytes: usize,
    ) -> Result<Self> {
        if !(1..=Self::MAX_RETAINED_MESSAGES).contains(&max_retained_messages) {
            return Err(MessagingError::Invalid {
                field: "retained message limit",
                reason: "must be within the documented configuration ceiling",
            });
        }
        if !(1..=Self::MAX_SUBSCRIPTIONS).contains(&max_subscriptions) {
            return Err(MessagingError::Invalid {
                field: "subscription limit",
                reason: "must be within the documented configuration ceiling",
            });
        }
        if !(1..=Self::MAX_ATTEMPTS).contains(&max_attempts) {
            return Err(MessagingError::Invalid {
                field: "delivery attempt limit",
                reason: "must be within the documented configuration ceiling",
            });
        }
        if !(1..=Self::MAX_PAYLOAD_BYTES).contains(&max_payload_bytes) {
            return Err(MessagingError::Invalid {
                field: "payload byte limit",
                reason: "must be within the documented configuration ceiling",
            });
        }
        self.max_retained_messages = max_retained_messages;
        self.max_subscriptions = max_subscriptions;
        self.max_attempts = max_attempts;
        self.max_payload_bytes = max_payload_bytes;
        Ok(self)
    }

    /// Returns the immutable namespace.
    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    /// Returns the retained-message bound.
    pub fn max_retained_messages(&self) -> usize {
        self.max_retained_messages
    }

    /// Returns the subscription bound.
    pub fn max_subscriptions(&self) -> usize {
        self.max_subscriptions
    }

    /// Returns the maximum delivery attempts before automatic dead-lettering.
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Returns the payload byte bound.
    pub fn max_payload_bytes(&self) -> usize {
        self.max_payload_bytes
    }
}
