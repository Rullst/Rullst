//! `begin_login` and `finish_login`: the two halves of a store-backed login.

use super::{
    ChallengeCookie, ChallengeError, ChallengeId, ChallengeStore, LoginChallenge,
    MAX_CHALLENGE_LIFETIME_SECONDS, ReturnPath,
};
use crate::configuration::{validate_authorization_url, validate_managed_query};
use crate::error::ConnectError;
use crate::pkce::{generate_oauth_state, generate_pkce};
use crate::provider::Provider;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Why a login could not start or finish. Messages contain no state, code,
/// verifier, nonce, challenge id or token.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LoginError {
    /// The callback request carried no usable challenge cookie.
    #[error("no login challenge cookie was presented")]
    MissingChallenge,
    /// The challenge store refused: unknown, replayed, altered or expired
    /// challenge, full store, or a storage failure.
    #[error(transparent)]
    Challenge(#[from] ChallengeError),
    /// The challenge was issued for another provider's login.
    #[error("the login challenge belongs to another provider")]
    ProviderMismatch,
    /// The callback `state` does not match the challenge.
    #[error("the callback state does not match the login challenge")]
    StateMismatch,
    /// The provider returned an OAuth error, such as `access_denied`. The
    /// code keeps at most 64 of its ASCII letters, digits, `_`, `-` and `.`.
    #[error("the provider refused the authorization: {0}")]
    Denied(String),
    /// The callback carried neither an error nor an authorization code.
    #[error("the callback did not contain an authorization code")]
    MissingCode,
    /// The code exchange or ID-token validation (including the nonce) failed.
    #[error("the provider exchange failed: {0}")]
    Exchange(ConnectError),
    /// The provider produced an unsafe authorization URL.
    #[error("invalid provider authorization request: {0}")]
    Configuration(ConnectError),
}

/// What [`begin_login`] stores for one attempt.
#[derive(Clone, Debug)]
pub struct LoginOptions {
    provider: String,
    oidc: bool,
    return_to: Option<ReturnPath>,
    lifetime_seconds: u64,
}

impl LoginOptions {
    /// An OAuth 2.0 login (state + PKCE) for the provider named `provider`,
    /// such as `github`; the callback passes the same name to `finish_login`.
    pub fn oauth(provider: impl Into<String>) -> Self {
        Self::new(provider, false)
    }

    /// An OpenID Connect login (state + PKCE + nonce), such as `google`. The
    /// ID token must then carry the stored nonce.
    pub fn oidc(provider: impl Into<String>) -> Self {
        Self::new(provider, true)
    }

    fn new(provider: impl Into<String>, oidc: bool) -> Self {
        Self {
            provider: provider.into(),
            oidc,
            return_to: None,
            lifetime_seconds: MAX_CHALLENGE_LIFETIME_SECONDS,
        }
    }

    /// The same-site page to open after login.
    #[must_use]
    pub fn with_return_to(mut self, return_to: ReturnPath) -> Self {
        self.return_to = Some(return_to);
        self
    }

    /// A lifetime shorter than the ten-minute default (1 to 600 whole seconds).
    #[must_use]
    pub fn with_lifetime(mut self, lifetime: Duration) -> Self {
        self.lifetime_seconds = lifetime.as_secs();
        self
    }
}

/// The provider redirect for a stored challenge.
///
/// The URL carries the state, PKCE challenge and nonce: redirect to it
/// without logging it. `Debug` redacts the URL and id.
#[derive(Clone)]
pub struct LoginRedirect {
    url: String,
    challenge_id: ChallengeId,
    expires_at: u64,
    lifetime_seconds: u64,
}

impl LoginRedirect {
    /// The provider authorization URL (in mock mode, the configured callback).
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The id to put in the challenge cookie.
    #[must_use]
    pub fn challenge_id(&self) -> &ChallengeId {
        &self.challenge_id
    }

    /// The Unix second from which the stored challenge is expired.
    #[must_use]
    pub fn expires_at_unix_seconds(&self) -> u64 {
        self.expires_at
    }

    /// The `Set-Cookie` value carrying the id for the challenge's lifetime.
    #[must_use]
    pub fn set_cookie(&self, cookie: &ChallengeCookie) -> String {
        cookie.set(&self.challenge_id, self.lifetime_seconds)
    }
}

impl fmt::Debug for LoginRedirect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoginRedirect")
            .field("url", &"[REDACTED]")
            .field("challenge_id", &self.challenge_id)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Generates the state, PKCE pair and (for OIDC) nonce, stores them and
/// returns the provider redirect with the challenge id.
///
/// Send the browser to [`LoginRedirect::url`] with
/// [`LoginRedirect::set_cookie`] in the same response.
pub async fn begin_login<S, P>(
    store: &S,
    provider: &P,
    options: LoginOptions,
) -> Result<LoginRedirect, LoginError>
where
    S: ChallengeStore,
    P: Provider + ?Sized,
{
    begin_login_at(store, provider, options, unix_now()?).await
}

pub(super) async fn begin_login_at<S, P>(
    store: &S,
    provider: &P,
    options: LoginOptions,
    now: u64,
) -> Result<LoginRedirect, LoginError>
where
    S: ChallengeStore,
    P: Provider + ?Sized,
{
    let lifetime_seconds = options.lifetime_seconds;
    let expires_at = now
        .checked_add(lifetime_seconds)
        .ok_or(ChallengeError::Invalid("lifetime"))?;
    let state = generate_oauth_state();
    let nonce = options.oidc.then(generate_oauth_state);
    let (code_verifier, code_challenge) = generate_pkce();
    let mut url = validate_authorization_url(
        &provider.redirect_url_with_pkce_and_state(&code_challenge, &state),
    )
    .map_err(LoginError::Configuration)?;
    validate_managed_query(&url, &state, &code_challenge).map_err(LoginError::Configuration)?;
    if let Some(nonce) = &nonce {
        url.query_pairs_mut().append_pair("nonce", nonce);
    }
    let challenge = LoginChallenge::new(
        options.provider,
        state,
        nonce,
        code_verifier,
        options.return_to,
        now,
        expires_at,
    )?;
    let challenge_id = store.save(&challenge).await?;
    Ok(LoginRedirect {
        url: url.into(),
        challenge_id,
        expires_at,
        lifetime_seconds,
    })
}

#[cfg(any(
    feature = "axum",
    feature = "actix",
    feature = "leptos",
    feature = "rullst"
))]
mod callback;
#[cfg(any(
    feature = "axum",
    feature = "actix",
    feature = "leptos",
    feature = "rullst"
))]
pub use callback::{CompletedLogin, finish_login};

fn unix_now() -> Result<u64, LoginError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|_| ChallengeError::Invalid("system clock").into())
}

#[cfg(test)]
#[path = "flow_tests.rs"]
mod tests;
