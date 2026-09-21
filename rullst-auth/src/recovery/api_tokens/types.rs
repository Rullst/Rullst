use super::*;
use std::collections::BTreeSet;
use zeroize::Zeroizing;

/// Nonempty exact scope names. No wildcard, hierarchy or implicit super-scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiScopes(BTreeSet<String>);
impl ApiScopes {
    pub fn new(values: impl IntoIterator<Item = impl Into<String>>) -> Result<Self, RecoveryError> {
        let mut scopes = BTreeSet::new();
        for value in values {
            let value = value.into();
            if scopes.len() >= 32
                || !(1..=64).contains(&value.len())
                || !value
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_lowercase())
                || !value.bytes().all(|b| {
                    b.is_ascii_lowercase()
                        || b.is_ascii_digit()
                        || matches!(b, b':' | b'_' | b'-' | b'.')
                })
                || !scopes.insert(value)
            {
                return Err(RecoveryError::InvalidInput);
            }
        }
        if scopes.is_empty() {
            return Err(RecoveryError::InvalidInput);
        }
        Ok(Self(scopes))
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }
    pub(super) fn includes(&self, required: &Self) -> bool {
        required.0.is_subset(&self.0)
    }
    pub(super) fn encode(&self) -> String {
        self.iter().collect::<Vec<_>>().join(",")
    }
    pub(super) fn decode(value: &str) -> Result<Self, RecoveryError> {
        if value.len() > 2079 {
            return Err(RecoveryError::Configuration);
        }
        let scopes = Self::new(value.split(',')).map_err(|_| RecoveryError::Configuration)?;
        if scopes.encode() != value {
            return Err(RecoveryError::Configuration);
        }
        Ok(scopes)
    }
}

/// Server-configured namespace/allowlist and hard storage/lifetime bounds.
#[derive(Clone)]
pub struct ApiTokenConfig {
    pub(super) namespace: String,
    pub(super) scopes: ApiScopes,
    pub(super) capacity: usize,
    pub(super) lifetime: u32,
}
impl std::fmt::Debug for ApiTokenConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiTokenConfig")
            .field("capacity", &self.capacity)
            .field("maximum_lifetime_seconds", &self.lifetime)
            .finish_non_exhaustive()
    }
}
impl ApiTokenConfig {
    pub fn new(
        namespace: impl Into<String>,
        allowed_scopes: ApiScopes,
        capacity: usize,
        maximum_lifetime_seconds: u32,
    ) -> Result<Self, RecoveryError> {
        let namespace = namespace.into();
        if !super::super::valid_subject(&namespace)
            || !(1..=100_000).contains(&capacity)
            || !(1..=30 * 86400).contains(&maximum_lifetime_seconds)
        {
            return Err(RecoveryError::Configuration);
        }
        Ok(Self {
            namespace,
            scopes: allowed_scopes,
            capacity,
            lifetime: maximum_lifetime_seconds,
        })
    }
}

/// Public management identifier, never a bearer credential or account selector.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiTokenId(pub(super) String);
impl ApiTokenId {
    pub fn new(value: impl Into<String>) -> Result<Self, RecoveryError> {
        let value = value.into();
        SecretToken::from_encoded(&value)?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Debug for ApiTokenId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiTokenId([REDACTED])")
    }
}

#[derive(Clone)]
pub struct ApiTokenMetadata {
    pub(super) id: ApiTokenId,
    pub(super) label: SessionLabel,
    pub(super) scopes: ApiScopes,
    pub(super) revision: i64,
    pub(super) created: i64,
    pub(super) issued: i64,
    pub(super) expires: i64,
}
impl ApiTokenMetadata {
    pub fn id(&self) -> &ApiTokenId {
        &self.id
    }
    pub fn label(&self) -> &SessionLabel {
        &self.label
    }
    pub fn scopes(&self) -> &ApiScopes {
        &self.scopes
    }
    pub fn revision(&self) -> u64 {
        self.revision as u64
    }
    pub fn created_at(&self) -> u64 {
        self.created as u64
    }
    pub fn issued_at(&self) -> u64 {
        self.issued as u64
    }
    pub fn expires_at(&self) -> u64 {
        self.expires as u64
    }
}
impl std::fmt::Debug for ApiTokenMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiTokenMetadata([REDACTED])")
    }
}

/// A one-time return value; persist the secret only in the caller's secret vault.
/// Neither this capability nor its Debug/metadata can recover another token.
pub struct IssuedApiToken {
    pub(super) bearer: Zeroizing<String>,
    pub(super) metadata: ApiTokenMetadata,
}
impl IssuedApiToken {
    pub fn expose_bearer(&self) -> &str {
        &self.bearer
    }
    pub fn metadata(&self) -> &ApiTokenMetadata {
        &self.metadata
    }
}
impl std::fmt::Debug for IssuedApiToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IssuedApiToken([REDACTED])")
    }
}

/// Database-verified token identity/scopes at one point in time. Resolve current
/// tenant membership, domain roles and resource ownership before acting. This
/// value cannot mint another token or revoke an account's other credentials.
#[derive(Clone)]
pub struct ApiTokenPrincipal {
    pub(super) subject: String,
    pub(super) namespace: String,
    pub(super) metadata: ApiTokenMetadata,
}
impl ApiTokenPrincipal {
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
    pub fn metadata(&self) -> &ApiTokenMetadata {
        &self.metadata
    }
}
impl std::fmt::Debug for ApiTokenPrincipal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiTokenPrincipal([REDACTED])")
    }
}

pub(super) fn bearer(id: &ApiTokenId) -> Result<Zeroizing<String>, RecoveryError> {
    let secret = SecretToken::generate()?;
    Ok(Zeroizing::new(format!(
        "rlt1_{}.{}",
        id.as_str(),
        secret.expose()
    )))
}
pub(super) fn parse_bearer(value: &str) -> Result<ApiTokenId, RecoveryError> {
    if value.len() != 92 {
        return Err(RecoveryError::InvalidAction);
    }
    let raw = value
        .strip_prefix("rlt1_")
        .ok_or(RecoveryError::InvalidAction)?;
    let (id, secret) = raw.split_once('.').ok_or(RecoveryError::InvalidAction)?;
    SecretToken::from_encoded(secret)?;
    ApiTokenId::new(id)
}
