use super::super::RecoveryError;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::fmt;

/// An account-scoped management reference, never an authentication credential.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionId(pub(super) String);

impl SessionId {
    /// Parses a reference returned by the current account's session inventory.
    /// Parsing does not authorize access to that session.
    pub fn new(value: impl Into<String>) -> Result<Self, RecoveryError> {
        let value = value.into();
        if value.len() != 43
            || URL_SAFE_NO_PAD.decode(&value).map_or(true, |bytes| {
                bytes.len() != 32 || URL_SAFE_NO_PAD.encode(bytes) != value
            })
        {
            return Err(RecoveryError::InvalidInput);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionId([REDACTED])")
    }
}

/// Optional display text supplied explicitly by the application or account owner.
/// It is not verified device identity; escape it when rendering HTML.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionLabel(String);

impl SessionLabel {
    /// Accepts 1–80 UTF-8 bytes without control characters or surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Result<Self, RecoveryError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 80
            || value.trim() != value
            || value.chars().any(char::is_control)
        {
            return Err(RecoveryError::InvalidInput);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SessionLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionLabel([REDACTED])")
    }
}

/// One active session of the currently authenticated account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveSession {
    pub(super) id: SessionId,
    pub(super) created_at: Option<u64>,
    pub(super) expires_at: u64,
    pub(super) label: Option<SessionLabel>,
    pub(super) current: bool,
}

impl ActiveSession {
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// Unix seconds; absent for a session created before metadata was introduced.
    pub fn created_at(&self) -> Option<u64> {
        self.created_at
    }

    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    pub fn label(&self) -> Option<&SessionLabel> {
        self.label.as_ref()
    }

    pub fn is_current(&self) -> bool {
        self.current
    }
}
