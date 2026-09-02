//! Strict application-issued JWT access tokens with key rotation and revocation hooks.
//!
//! These tokens are intentionally separate from third-party OAuth/OIDC tokens.
//! Production policies require a revocation store that reports shared durability;
//! the bundled in-memory store is deterministic and process-local for development.

use crate::validate_app_key;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_TOKEN_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const MAX_CLOCK_SKEW: Duration = Duration::from_secs(5 * 60);
const MAX_IDENTITY_BYTES: usize = 256;
const MAX_SCOPES: usize = 64;
const MAX_SCOPE_BYTES: usize = 128;
const MAX_KEYS: usize = 8;

mod async_store;
pub use async_store::{AsyncJwtRevocationStore, JwtRevocationMode};
#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteJwtRevocationSnapshot, SqliteJwtRevocationStore};

/// Failure domain for application JWT policy, issuance and verification.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum JwtError {
    #[error("invalid JWT configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("JWT signing keys must satisfy the APP_KEY strength policy")]
    WeakSigningKey,
    #[error("JWT time-to-live is outside the configured policy")]
    InvalidTimeToLive,
    #[error("JWT system time cannot be represented")]
    InvalidSystemTime,
    #[error("JWT encoding failed: {0}")]
    Encoding(String),
    #[error("JWT is invalid or expired")]
    InvalidToken,
    #[error("JWT has been revoked")]
    Revoked,
    #[error("production JWT verification requires a shared revocation store")]
    RevocationStoreNotShared,
    #[error("JWT revocation store reached its configured capacity")]
    RevocationStoreCapacity,
    #[error("JWT revocation backend failed: {0}")]
    RevocationBackend(String),
}

/// Deployment posture enforced when verifying an application JWT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JwtPolicyMode {
    Development,
    Production,
}

/// One versioned symmetric signing key. `kid` is public; `secret` is never serialized.
#[derive(Clone)]
pub struct JwtSigningKey {
    kid: String,
    secret: Vec<u8>,
}

impl JwtSigningKey {
    pub fn new(kid: impl Into<String>, secret: impl AsRef<[u8]>) -> Result<Self, JwtError> {
        let kid = kid.into();
        if !valid_identifier(&kid, 64) {
            return Err(JwtError::InvalidConfiguration("kid"));
        }
        let secret = secret.as_ref().to_vec();
        validate_app_key(&secret).map_err(|_| JwtError::WeakSigningKey)?;
        Ok(Self { kid, secret })
    }

    pub fn kid(&self) -> &str {
        &self.kid
    }
}

/// Versioned claims issued only by [`ApplicationJwtPolicy`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationJwtClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub iat: u64,
    pub nbf: u64,
    pub exp: u64,
    pub jti: String,
    pub session_version: u64,
    pub scopes: Vec<String>,
    pub token_use: String,
    pub schema_version: u8,
}

/// Static-dispatch hook for durable application-owned revocation backends.
pub trait JwtRevocationStore: Send + Sync {
    fn mode(&self) -> JwtRevocationMode;

    fn is_revoked(&self, claims: &ApplicationJwtClaims, now: u64) -> Result<bool, JwtError>;
}

/// Bounded deterministic revocation state for development and single-process tests.
pub struct InMemoryJwtRevocationStore {
    state: Mutex<InMemoryRevocationState>,
    max_entries: usize,
}

#[derive(Default)]
struct InMemoryRevocationState {
    revoked_tokens: HashMap<String, u64>,
    subject_versions: HashMap<String, u64>,
}

impl InMemoryJwtRevocationStore {
    pub fn new(max_entries: usize) -> Result<Self, JwtError> {
        if !(1..=1_000_000).contains(&max_entries) {
            return Err(JwtError::InvalidConfiguration("max_entries"));
        }
        Ok(Self {
            state: Mutex::new(InMemoryRevocationState::default()),
            max_entries,
        })
    }

    pub fn revoke_token(&self, claims: &ApplicationJwtClaims) -> Result<(), JwtError> {
        if !valid_identifier(&claims.jti, 64) {
            return Err(JwtError::InvalidConfiguration("jti"));
        }
        let now = unix_time()?;
        if claims.exp <= now {
            return Ok(());
        }
        let mut state = self.lock_state()?;
        state
            .revoked_tokens
            .retain(|_, expires_at| *expires_at > now);
        if !state.revoked_tokens.contains_key(&claims.jti)
            && Self::entry_count_locked(&state) >= self.max_entries
        {
            return Err(JwtError::RevocationStoreCapacity);
        }
        state.revoked_tokens.insert(claims.jti.clone(), claims.exp);
        Ok(())
    }

    /// Rejects subject tokens whose `session_version` is lower than this value.
    pub fn revoke_subject_before(
        &self,
        subject: impl Into<String>,
        minimum_session_version: u64,
    ) -> Result<(), JwtError> {
        let subject = subject.into();
        if !valid_identity(&subject) || minimum_session_version == 0 {
            return Err(JwtError::InvalidConfiguration("subject revocation"));
        }
        let mut state = self.lock_state()?;
        if !state.subject_versions.contains_key(&subject)
            && Self::entry_count_locked(&state) >= self.max_entries
        {
            return Err(JwtError::RevocationStoreCapacity);
        }
        state
            .subject_versions
            .entry(subject)
            .and_modify(|version| *version = (*version).max(minimum_session_version))
            .or_insert(minimum_session_version);
        Ok(())
    }

    pub fn entry_count(&self) -> Result<usize, JwtError> {
        let state = self.lock_state()?;
        Ok(Self::entry_count_locked(&state))
    }

    fn entry_count_locked(state: &InMemoryRevocationState) -> usize {
        state.revoked_tokens.len() + state.subject_versions.len()
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, InMemoryRevocationState>, JwtError> {
        self.state
            .lock()
            .map_err(|_| JwtError::RevocationBackend("in-memory lock poisoned".to_string()))
    }
}

impl JwtRevocationStore for InMemoryJwtRevocationStore {
    fn mode(&self) -> JwtRevocationMode {
        JwtRevocationMode::ProcessLocal
    }

    fn is_revoked(&self, claims: &ApplicationJwtClaims, now: u64) -> Result<bool, JwtError> {
        let mut state = self.lock_state()?;
        state
            .revoked_tokens
            .retain(|_, expires_at| *expires_at > now);
        if state.revoked_tokens.contains_key(&claims.jti) {
            return Ok(true);
        }
        Ok(state
            .subject_versions
            .get(&claims.sub)
            .is_some_and(|minimum| claims.session_version < *minimum))
    }
}

/// Issuer/verifier policy with one active key and bounded previous verification keys.
pub struct ApplicationJwtPolicy {
    issuer: String,
    audience: String,
    max_ttl: Duration,
    clock_skew: Duration,
    mode: JwtPolicyMode,
    active_kid: String,
    keys: HashMap<String, Vec<u8>>,
}

impl ApplicationJwtPolicy {
    pub fn production(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        max_ttl: Duration,
        active_key: JwtSigningKey,
    ) -> Result<Self, JwtError> {
        Self::build(
            issuer.into(),
            audience.into(),
            max_ttl,
            JwtPolicyMode::Production,
            active_key,
        )
    }

    pub fn development(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        max_ttl: Duration,
        active_key: JwtSigningKey,
    ) -> Result<Self, JwtError> {
        Self::build(
            issuer.into(),
            audience.into(),
            max_ttl,
            JwtPolicyMode::Development,
            active_key,
        )
    }

    fn build(
        issuer: String,
        audience: String,
        max_ttl: Duration,
        mode: JwtPolicyMode,
        active_key: JwtSigningKey,
    ) -> Result<Self, JwtError> {
        if !valid_identity(&issuer) {
            return Err(JwtError::InvalidConfiguration("issuer"));
        }
        if !valid_identity(&audience) {
            return Err(JwtError::InvalidConfiguration("audience"));
        }
        if max_ttl.is_zero() || max_ttl > MAX_TOKEN_TTL {
            return Err(JwtError::InvalidConfiguration("max_ttl"));
        }
        let active_kid = active_key.kid.clone();
        let keys = HashMap::from([(active_key.kid, active_key.secret)]);
        Ok(Self {
            issuer,
            audience,
            max_ttl,
            clock_skew: Duration::from_secs(30),
            mode,
            active_kid,
            keys,
        })
    }

    pub fn with_clock_skew(mut self, clock_skew: Duration) -> Result<Self, JwtError> {
        if clock_skew > MAX_CLOCK_SKEW {
            return Err(JwtError::InvalidConfiguration("clock_skew"));
        }
        self.clock_skew = clock_skew;
        Ok(self)
    }

    /// Makes `new_key` active while retaining existing keys for verification.
    pub fn rotate(mut self, new_key: JwtSigningKey) -> Result<Self, JwtError> {
        if self.keys.contains_key(&new_key.kid) {
            return Err(JwtError::InvalidConfiguration("duplicate kid"));
        }
        if self.keys.len() >= MAX_KEYS {
            return Err(JwtError::InvalidConfiguration("too many keys"));
        }
        self.active_kid = new_key.kid.clone();
        self.keys.insert(new_key.kid, new_key.secret);
        Ok(self)
    }

    pub fn issue<S, I>(
        &self,
        subject: impl Into<String>,
        scopes: I,
        session_version: u64,
        ttl: Duration,
    ) -> Result<String, JwtError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.issue_at(subject.into(), scopes, session_version, ttl, unix_time()?)
    }

    fn issue_at<S, I>(
        &self,
        subject: String,
        scopes: I,
        session_version: u64,
        ttl: Duration,
        issued_at: u64,
    ) -> Result<String, JwtError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        if !valid_identity(&subject) || session_version == 0 {
            return Err(JwtError::InvalidConfiguration("subject"));
        }
        if ttl.is_zero() || ttl > self.max_ttl {
            return Err(JwtError::InvalidTimeToLive);
        }
        let scopes = normalized_scopes(scopes)?;
        let expires_at = issued_at
            .checked_add(ttl.as_secs())
            .ok_or(JwtError::InvalidSystemTime)?;
        let mut random_id = [0_u8; 16];
        rand::fill(&mut random_id);
        let claims = ApplicationJwtClaims {
            sub: subject,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: issued_at,
            nbf: issued_at,
            exp: expires_at,
            jti: hex::encode(random_id),
            session_version,
            scopes,
            token_use: "access".to_string(),
            schema_version: 1,
        };
        let key = self
            .keys
            .get(&self.active_kid)
            .ok_or(JwtError::InvalidConfiguration("active key"))?;
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(self.active_kid.clone());
        encode(&header, &claims, &EncodingKey::from_secret(key))
            .map_err(|error| JwtError::Encoding(error.to_string()))
    }

    pub fn verify<R: JwtRevocationStore>(
        &self,
        token: &str,
        revocations: &R,
    ) -> Result<ApplicationJwtClaims, JwtError> {
        if self.mode == JwtPolicyMode::Production && revocations.mode() != JwtRevocationMode::Shared
        {
            return Err(JwtError::RevocationStoreNotShared);
        }
        let now = unix_time()?;
        let claims = self.decode_and_validate(token, now)?;
        if revocations.is_revoked(&claims, now)? {
            return Err(JwtError::Revoked);
        }
        Ok(claims)
    }

    /// Verifies an application token against an async shared revocation backend.
    pub async fn verify_async<R: AsyncJwtRevocationStore>(
        &self,
        token: &str,
        revocations: &R,
    ) -> Result<ApplicationJwtClaims, JwtError> {
        if self.mode == JwtPolicyMode::Production && revocations.mode() != JwtRevocationMode::Shared
        {
            return Err(JwtError::RevocationStoreNotShared);
        }
        let now = unix_time()?;
        let claims = self.decode_and_validate(token, now)?;
        if revocations.is_revoked(&claims, now).await? {
            return Err(JwtError::Revoked);
        }
        Ok(claims)
    }

    fn decode_and_validate(&self, token: &str, now: u64) -> Result<ApplicationJwtClaims, JwtError> {
        if token.len() > 16 * 1024 {
            return Err(JwtError::InvalidToken);
        }
        let header = decode_header(token).map_err(|_| JwtError::InvalidToken)?;
        if header.alg != Algorithm::HS256 || header.typ.as_deref() != Some("JWT") {
            return Err(JwtError::InvalidToken);
        }
        let kid = header.kid.ok_or(JwtError::InvalidToken)?;
        let key = self.keys.get(&kid).ok_or(JwtError::InvalidToken)?;
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[self.audience.as_str()]);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_required_spec_claims(&["aud", "exp", "iat", "iss", "jti", "nbf", "sub"]);
        validation.leeway = self.clock_skew.as_secs();
        validation.validate_exp = true;
        validation.validate_nbf = true;
        let claims =
            decode::<ApplicationJwtClaims>(token, &DecodingKey::from_secret(key), &validation)
                .map_err(|_| JwtError::InvalidToken)?
                .claims;
        self.validate_claims(&claims, now)?;
        Ok(claims)
    }

    fn validate_claims(&self, claims: &ApplicationJwtClaims, now: u64) -> Result<(), JwtError> {
        let maximum_iat = now.saturating_add(self.clock_skew.as_secs());
        let ttl = claims
            .exp
            .checked_sub(claims.iat)
            .ok_or(JwtError::InvalidToken)?;
        if claims.schema_version != 1
            || claims.token_use != "access"
            || !valid_identity(&claims.sub)
            || !valid_identifier(&claims.jti, 64)
            || claims.session_version == 0
            || claims.iat > maximum_iat
            || claims.nbf != claims.iat
            || ttl == 0
            || ttl > self.max_ttl.as_secs()
            || normalized_scopes(claims.scopes.clone()).map_err(|_| JwtError::InvalidToken)?
                != claims.scopes
        {
            return Err(JwtError::InvalidToken);
        }
        Ok(())
    }
}

fn unix_time() -> Result<u64, JwtError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| JwtError::InvalidSystemTime)
        .map(|duration| duration.as_secs())
}

fn normalized_scopes<S, I>(scopes: I) -> Result<Vec<String>, JwtError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut scopes = scopes.into_iter().map(Into::into).collect::<Vec<_>>();
    if scopes.len() > MAX_SCOPES
        || scopes.iter().any(|scope| {
            !valid_identifier(scope, MAX_SCOPE_BYTES) || scope.starts_with(['-', '_', ':', '.'])
        })
    {
        return Err(JwtError::InvalidConfiguration("scopes"));
    }
    scopes.sort_unstable();
    if scopes.iter().collect::<HashSet<_>>().len() != scopes.len() {
        return Err(JwtError::InvalidConfiguration("duplicate scopes"));
    }
    Ok(scopes)
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.len() <= MAX_IDENTITY_BYTES
        && !value.chars().any(char::is_control)
}

fn valid_identifier(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

#[cfg(test)]
mod tests;
