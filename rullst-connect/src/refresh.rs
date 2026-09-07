//! Bounded, process-local coordination for automatic OAuth token refresh.

mod snapshot;
#[cfg(feature = "sqlite")]
mod sqlite;

pub use snapshot::{
    EncryptedTokenSnapshot, TokenSnapshotBinding, TokenSnapshotError, TokenSnapshotKey,
};
#[cfg(feature = "sqlite")]
pub use sqlite::{
    SqliteTokenSnapshotStore, TokenStoreError, TokenStoreMetadata, TokenStoreSnapshot,
};

use crate::{ConnectError, ConnectUser, Provider};
use secrecy::{ExposeSecret, SecretString};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const MAX_TOKEN_BYTES: usize = 64 * 1024;
pub(crate) const MAX_TOKEN_LIFETIME_SECONDS: u64 = 366 * 24 * 60 * 60;
const MAX_REFRESH_LEEWAY_SECONDS: u64 = 60 * 60;
const DEFAULT_REFRESH_LEEWAY_SECONDS: u64 = 60;

/// One validated access/refresh token generation suitable for encrypted storage.
///
/// This value deliberately does not implement Serde. Applications that persist
/// it must use a dedicated encrypted secret store and bind it to the owning
/// account and provider.
#[derive(Clone)]
#[non_exhaustive]
pub struct RefreshableTokenState {
    provider_user_id: String,
    access_token: SecretString,
    refresh_token: SecretString,
    issued_at: u64,
    expires_at: u64,
    generation: u64,
}

impl RefreshableTokenState {
    /// Creates the first validated token generation.
    pub fn try_new(
        provider_user_id: impl Into<String>,
        access_token: SecretString,
        refresh_token: SecretString,
        issued_at: u64,
        expires_in: u64,
    ) -> Result<Self, ConnectError> {
        let provider_user_id = provider_user_id.into();
        validate_provider_user_id(&provider_user_id)?;
        validate_token("access token", &access_token)?;
        validate_token("refresh token", &refresh_token)?;
        let expires_at = validate_expiration(issued_at, expires_in)?;
        Ok(Self {
            provider_user_id,
            access_token,
            refresh_token,
            issued_at,
            expires_at,
            generation: 0,
        })
    }

    /// Builds state from a provider result received at an explicit trusted time.
    pub fn from_user_at(user: &ConnectUser, issued_at: u64) -> Result<Self, ConnectError> {
        let refresh_token = user.refresh_token.clone().ok_or_else(|| {
            ConnectError::Token(
                "automatic refresh requires a provider-issued refresh token".to_string(),
            )
        })?;
        let expires_in = user.expires_in.ok_or_else(|| {
            ConnectError::Token(
                "automatic refresh requires a positive provider token lifetime".to_string(),
            )
        })?;
        Self::try_new(
            user.id.clone(),
            user.access_token.clone(),
            refresh_token,
            issued_at,
            expires_in,
        )
    }

    /// Returns the provider user ID bound to every refreshed generation.
    pub fn provider_user_id(&self) -> &str {
        &self.provider_user_id
    }

    /// Returns the access token as a redacting secret value.
    pub fn access_token(&self) -> &SecretString {
        &self.access_token
    }

    /// Returns the refresh token for application-owned encrypted persistence.
    pub fn refresh_token(&self) -> &SecretString {
        &self.refresh_token
    }

    /// Returns the trusted Unix time at which this generation was received.
    pub fn issued_at(&self) -> u64 {
        self.issued_at
    }

    /// Returns the computed Unix expiration time.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Returns the process-local successful-refresh generation.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn should_refresh(&self, now: u64, requested_leeway: u64) -> bool {
        let lifetime = self.expires_at.saturating_sub(self.issued_at);
        let effective_leeway = requested_leeway.min(lifetime.saturating_sub(1));
        now >= self.expires_at.saturating_sub(effective_leeway)
    }

    fn from_refresh(
        user: &ConnectUser,
        expected_provider_user_id: &str,
        prior_refresh_token: &SecretString,
        issued_at: u64,
        generation: u64,
    ) -> Result<Self, ConnectError> {
        if user.id != expected_provider_user_id {
            return Err(ConnectError::Token(
                "refreshed token response changed provider user identity".to_string(),
            ));
        }
        let refresh_token = user
            .refresh_token
            .clone()
            .unwrap_or_else(|| prior_refresh_token.clone());
        let expires_in = user.expires_in.ok_or_else(|| {
            ConnectError::Token("refreshed token response omitted a positive lifetime".to_string())
        })?;
        let mut state = Self::try_new(
            user.id.clone(),
            user.access_token.clone(),
            refresh_token,
            issued_at,
            expires_in,
        )?;
        state.generation = generation;
        Ok(state)
    }

    fn try_restore(
        provider_user_id: String,
        access_token: String,
        refresh_token: String,
        issued_at: u64,
        expires_at: u64,
        generation: u64,
    ) -> Result<Self, ConnectError> {
        let access_token = SecretString::from(access_token);
        let refresh_token = SecretString::from(refresh_token);
        validate_provider_user_id(&provider_user_id)?;
        validate_token("access token", &access_token)?;
        validate_token("refresh token", &refresh_token)?;
        if expires_at <= issued_at
            || expires_at.saturating_sub(issued_at) > MAX_TOKEN_LIFETIME_SECONDS
        {
            return Err(ConnectError::Token(
                "restored token expiration is outside the supported lifetime".to_string(),
            ));
        }
        Ok(Self {
            provider_user_id,
            access_token,
            refresh_token,
            issued_at,
            expires_at,
            generation,
        })
    }
}

impl std::fmt::Debug for RefreshableTokenState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RefreshableTokenState")
            .field("provider_user_id", &"[REDACTED]")
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .field("issued_at", &self.issued_at)
            .field("expires_at", &self.expires_at)
            .field("generation", &self.generation)
            .finish()
    }
}

/// A redacting access-token lease returned to the application.
#[derive(Clone)]
#[non_exhaustive]
pub struct AccessTokenLease {
    access_token: SecretString,
    expires_at: u64,
    generation: u64,
    refreshed: bool,
}

impl AccessTokenLease {
    /// Returns the access token secret.
    pub fn access_token(&self) -> &SecretString {
        &self.access_token
    }

    /// Returns the token's Unix expiration time.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Returns the token generation observed by this lease.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Reports whether this call performed the provider refresh.
    pub fn was_refreshed(&self) -> bool {
        self.refreshed
    }
}

impl std::fmt::Debug for AccessTokenLease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AccessTokenLease")
            .field("access_token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .field("generation", &self.generation)
            .field("refreshed", &self.refreshed)
            .finish()
    }
}

/// Process-local, statically dispatched automatic refresh coordinator.
///
/// A Tokio mutex serializes refresh calls and the state changes only after a
/// complete, validated provider response. Cross-process leases, encrypted
/// persistence and account authorization remain application responsibilities.
pub struct AutoRefreshingSession<'provider, SelectedProvider>
where
    SelectedProvider: Provider + ?Sized,
{
    provider: &'provider SelectedProvider,
    state: Mutex<RefreshableTokenState>,
    refresh_leeway_seconds: u64,
}

impl<'provider, SelectedProvider> AutoRefreshingSession<'provider, SelectedProvider>
where
    SelectedProvider: Provider + ?Sized,
{
    /// Creates a coordinator from validated token state.
    pub fn new(provider: &'provider SelectedProvider, state: RefreshableTokenState) -> Self {
        Self {
            provider,
            state: Mutex::new(state),
            refresh_leeway_seconds: DEFAULT_REFRESH_LEEWAY_SECONDS,
        }
    }

    /// Creates a coordinator from a provider response and explicit trusted time.
    pub fn from_user_at(
        provider: &'provider SelectedProvider,
        user: &ConnectUser,
        issued_at: u64,
    ) -> Result<Self, ConnectError> {
        Ok(Self::new(
            provider,
            RefreshableTokenState::from_user_at(user, issued_at)?,
        ))
    }

    /// Creates a coordinator using the current system clock.
    pub fn from_user(
        provider: &'provider SelectedProvider,
        user: &ConnectUser,
    ) -> Result<Self, ConnectError> {
        Self::from_user_at(provider, user, unix_now()?)
    }

    /// Configures an early-refresh window of at most one hour.
    pub fn with_refresh_leeway(mut self, seconds: u64) -> Result<Self, ConnectError> {
        if seconds > MAX_REFRESH_LEEWAY_SECONDS {
            return Err(ConnectError::Token(format!(
                "refresh leeway cannot exceed {MAX_REFRESH_LEEWAY_SECONDS} seconds"
            )));
        }
        self.refresh_leeway_seconds = seconds;
        Ok(self)
    }

    /// Returns a valid lease, refreshing once when the current token is due.
    pub async fn access_token(&self) -> Result<AccessTokenLease, ConnectError> {
        self.access_token_at(unix_now()?).await
    }

    /// Uses an explicit trusted clock for deterministic workers and tests.
    pub async fn access_token_at(&self, now: u64) -> Result<AccessTokenLease, ConnectError> {
        let mut state = self.state.lock().await;
        if !state.should_refresh(now, self.refresh_leeway_seconds) {
            return Ok(lease_from(&state, false));
        }

        let next_generation = state.generation.checked_add(1).ok_or_else(|| {
            ConnectError::Token("automatic refresh generation overflowed".to_string())
        })?;
        let refreshed = self
            .provider
            .refresh_token(state.refresh_token.expose_secret())
            .await?;
        let replacement = RefreshableTokenState::from_refresh(
            &refreshed,
            &state.provider_user_id,
            &state.refresh_token,
            now,
            next_generation,
        )?;
        *state = replacement;
        Ok(lease_from(&state, true))
    }

    /// Clones the current redacting state for encrypted persistence.
    pub async fn state_snapshot(&self) -> RefreshableTokenState {
        self.state.lock().await.clone()
    }
}

impl<SelectedProvider> std::fmt::Debug for AutoRefreshingSession<'_, SelectedProvider>
where
    SelectedProvider: Provider + ?Sized,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AutoRefreshingSession")
            .field("provider", &std::any::type_name::<SelectedProvider>())
            .field("state", &"[REDACTED]")
            .field("refresh_leeway_seconds", &self.refresh_leeway_seconds)
            .finish()
    }
}

fn lease_from(state: &RefreshableTokenState, refreshed: bool) -> AccessTokenLease {
    AccessTokenLease {
        access_token: state.access_token.clone(),
        expires_at: state.expires_at,
        generation: state.generation,
        refreshed,
    }
}

fn validate_token(label: &str, token: &SecretString) -> Result<(), ConnectError> {
    let value = token.expose_secret();
    if value.is_empty() || value.len() > MAX_TOKEN_BYTES || value.chars().any(char::is_control) {
        return Err(ConnectError::Token(format!(
            "{label} must contain 1 to {MAX_TOKEN_BYTES} non-control bytes"
        )));
    }
    Ok(())
}

fn validate_provider_user_id(value: &str) -> Result<(), ConnectError> {
    if value.is_empty()
        || value.len() > 512
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(ConnectError::Token(
            "provider user ID must contain 1 to 512 non-control bytes without surrounding whitespace"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_expiration(issued_at: u64, expires_in: u64) -> Result<u64, ConnectError> {
    if expires_in == 0 || expires_in > MAX_TOKEN_LIFETIME_SECONDS {
        return Err(ConnectError::Token(format!(
            "token lifetime must contain 1 to {MAX_TOKEN_LIFETIME_SECONDS} seconds"
        )));
    }
    issued_at
        .checked_add(expires_in)
        .ok_or_else(|| ConnectError::Time("token expiration overflowed".to_string()))
}

fn unix_now() -> Result<u64, ConnectError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[cfg(test)]
#[path = "refresh_tests.rs"]
mod tests;
