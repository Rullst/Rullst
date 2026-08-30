//! Short-lived OAuth/OIDC challenge lifecycle for tower-sessions.

use super::AuthCallback;
use crate::error::ConnectError;
use crate::pkce::{generate_oauth_state, generate_pkce};
use crate::provider::{ExchangeParams, Provider};
use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tower_sessions::Session;

const CHALLENGE_KEY: &str = "rullst_oauth_challenge_v1";
const LEGACY_STATE_KEY: &str = "oauth_state";
const CHALLENGE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredChallenge {
    state: String,
    nonce: Option<String>,
    code_verifier: String,
    expires_at_unix_seconds: u64,
}

/// A short-lived authorization redirect created from a server-side session challenge.
///
/// The URL contains state, PKCE challenge and, for OIDC, nonce. Treat it as transient sensitive
/// data and redirect it without logging. The PKCE verifier remains only in the server-side session.
#[derive(Clone)]
pub struct OAuthAuthorization {
    url: String,
    expires_at_unix_seconds: u64,
}

impl OAuthAuthorization {
    /// Returns the provider authorization URL to use for the redirect.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the absolute Unix expiration of the stored challenge.
    pub fn expires_at_unix_seconds(&self) -> u64 {
        self.expires_at_unix_seconds
    }
}

impl fmt::Debug for OAuthAuthorization {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuthAuthorization")
            .field("url", &"[REDACTED]")
            .field("expires_at_unix_seconds", &self.expires_at_unix_seconds)
            .finish()
    }
}

/// Generates and stores a ten-minute OAuth 2.0 state + PKCE challenge.
///
/// Only one challenge can be active per tower session. Starting another flow replaces the
/// previous one. The callback path removes and saves it before validation, so a later sequential
/// callback cannot reuse it. The generic session-store trait does not provide distributed
/// compare-and-delete for already-loaded concurrent requests. Use [`begin_oidc_session`] when the
/// provider returns an ID token.
pub async fn begin_oauth_session<P>(
    session: &Session,
    provider: &P,
) -> Result<OAuthAuthorization, ConnectError>
where
    P: Provider + ?Sized,
{
    begin_session(session, provider, false).await
}

/// Generates and stores a ten-minute OIDC state + PKCE + nonce challenge.
///
/// The returned URL includes the nonce. The callback extractor later exposes that exact nonce
/// through [`AuthSession::exchange_params`] for cryptographic ID-token validation.
pub async fn begin_oidc_session<P>(
    session: &Session,
    provider: &P,
) -> Result<OAuthAuthorization, ConnectError>
where
    P: Provider + ?Sized,
{
    begin_session(session, provider, true).await
}

async fn begin_session<P>(
    session: &Session,
    provider: &P,
    include_nonce: bool,
) -> Result<OAuthAuthorization, ConnectError>
where
    P: Provider + ?Sized,
{
    let state = generate_oauth_state();
    let nonce = include_nonce.then(generate_oauth_state);
    let (code_verifier, code_challenge) = generate_pkce();
    let now = unix_seconds(SystemTime::now())?;
    let expires_at_unix_seconds = now.checked_add(CHALLENGE_TTL.as_secs()).ok_or_else(|| {
        ConnectError::Time("OAuth challenge expiration exceeds Unix time range".to_string())
    })?;
    let mut url = crate::configuration::validate_authorization_url(
        &provider.redirect_url_with_pkce_and_state(&code_challenge, &state),
    )?;
    validate_managed_query(&url, &state, &code_challenge)?;
    if let Some(nonce) = &nonce {
        url.query_pairs_mut().append_pair("nonce", nonce);
    }

    session
        .remove::<String>(LEGACY_STATE_KEY)
        .await
        .map_err(session_error)?;
    session
        .insert(
            CHALLENGE_KEY,
            StoredChallenge {
                state,
                nonce,
                code_verifier,
                expires_at_unix_seconds,
            },
        )
        .await
        .map_err(session_error)?;

    Ok(OAuthAuthorization {
        url: url.to_string(),
        expires_at_unix_seconds,
    })
}

/// Validated callback plus the consumed OIDC nonce and PKCE verifier, when present.
#[derive(Clone)]
pub struct AuthSession {
    /// Real callback parameters parsed from the query string.
    pub callback: AuthCallback,
    expected_nonce: Option<String>,
    code_verifier: Option<String>,
}

impl AuthSession {
    /// Returns the consumed expected OIDC nonce, if the flow was started as OIDC.
    pub fn expected_nonce(&self) -> Option<&str> {
        self.expected_nonce.as_deref()
    }

    /// Returns the consumed PKCE verifier. Legacy state-only sessions return `None`.
    pub fn code_verifier(&self) -> Option<&str> {
        self.code_verifier.as_deref()
    }

    /// Builds the exact parameters for the provider token and ID-token exchange.
    pub fn exchange_params(&self) -> Result<ExchangeParams<'_>, ConnectError> {
        if let Some(error) = &self.callback.error {
            return Err(ConnectError::Provider(format!(
                "authorization server returned error: {}",
                bounded_error(error)
            )));
        }
        let auth_code = self.callback.code.as_deref().ok_or_else(|| {
            ConnectError::Token("authorization callback did not contain a code".to_string())
        })?;
        Ok(ExchangeParams {
            auth_code,
            code_verifier: self.code_verifier(),
            expected_nonce: self.expected_nonce(),
        })
    }
}

impl fmt::Debug for AuthSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthSession")
            .field("callback", &"[REDACTED]")
            .field("has_expected_nonce", &self.expected_nonce.is_some())
            .field("has_code_verifier", &self.code_verifier.is_some())
            .finish()
    }
}

impl<S> FromRequestParts<S> for AuthSession
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let session = parts
            .extensions
            .get::<Session>()
            .cloned()
            .ok_or_else(|| internal_response("Missing tower-sessions extension"))?;
        let axum::extract::Query(callback) =
            axum::extract::Query::<AuthCallback>::from_request_parts(parts, state)
                .await
                .map_err(IntoResponse::into_response)?;

        let challenge = session
            .remove::<StoredChallenge>(CHALLENGE_KEY)
            .await
            .map_err(|_| internal_response("OAuth session challenge is unavailable"))?;
        if let Some(challenge) = challenge {
            session
                .save()
                .await
                .map_err(|_| internal_response("OAuth challenge consumption could not be saved"))?;
            let callback_state = callback
                .state
                .as_deref()
                .ok_or_else(|| bad_request("Missing CSRF state parameter"))?;
            validate_challenge(callback_state, &challenge)
                .map_err(|(status, message)| (status, message).into_response())?;
            return Ok(Self {
                callback,
                expected_nonce: challenge.nonce,
                code_verifier: Some(challenge.code_verifier),
            });
        }

        let legacy_state = session
            .remove::<String>(LEGACY_STATE_KEY)
            .await
            .map_err(|_| internal_response("Legacy OAuth session state is unavailable"))?;
        let Some(legacy_state) = legacy_state else {
            return Err(bad_request("CSRF state mismatch"));
        };
        session
            .save()
            .await
            .map_err(|_| internal_response("Legacy OAuth state consumption could not be saved"))?;
        callback
            .state
            .as_deref()
            .ok_or_else(|| bad_request("Missing CSRF state parameter"))?;
        callback
            .verify_state(&legacy_state)
            .map_err(|_| bad_request("CSRF state mismatch"))?;
        Ok(Self {
            callback,
            expected_nonce: None,
            code_verifier: None,
        })
    }
}

fn validate_challenge(
    callback_state: &str,
    challenge: &StoredChallenge,
) -> Result<(), (axum::http::StatusCode, &'static str)> {
    let now = unix_seconds(SystemTime::now()).map_err(|_| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "System time is unavailable",
        )
    })?;
    if now >= challenge.expires_at_unix_seconds {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "OAuth session challenge expired",
        ));
    }
    AuthCallback {
        code: None,
        state: Some(callback_state.to_string()),
        error: None,
        error_description: None,
    }
    .verify_state(&challenge.state)
    .map_err(|_| (axum::http::StatusCode::BAD_REQUEST, "CSRF state mismatch"))
}

fn unix_seconds(now: SystemTime) -> Result<u64, ConnectError> {
    Ok(now.duration_since(UNIX_EPOCH)?.as_secs())
}

fn validate_managed_query(
    url: &url::Url,
    expected_state: &str,
    expected_challenge: &str,
) -> Result<(), ConnectError> {
    let mut state_matches = 0_u8;
    let mut challenge_matches = 0_u8;
    let mut method_matches = 0_u8;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "state" if value == expected_state => state_matches = state_matches.saturating_add(1),
            "code_challenge" if value == expected_challenge => {
                challenge_matches = challenge_matches.saturating_add(1);
            }
            "code_challenge_method" if value == "S256" => {
                method_matches = method_matches.saturating_add(1);
            }
            "state" | "code_challenge" | "code_challenge_method" | "nonce" => {
                return Err(ConnectError::InvalidConfiguration {
                    field: "authorization_url",
                    reason: "provider returned a conflicting managed OAuth parameter".to_string(),
                });
            }
            _ => {}
        }
    }
    if state_matches == 1 && challenge_matches == 1 && method_matches == 1 {
        Ok(())
    } else {
        Err(ConnectError::InvalidConfiguration {
            field: "authorization_url",
            reason: "provider did not preserve exactly one state and S256 PKCE challenge"
                .to_string(),
        })
    }
}

fn session_error(_error: tower_sessions::session::Error) -> ConnectError {
    ConnectError::Session("OAuth session store operation failed".to_string())
}

fn bounded_error(error: &str) -> String {
    error.chars().take(128).collect()
}

fn bad_request(message: &'static str) -> Response {
    (axum::http::StatusCode::BAD_REQUEST, message).into_response()
}

fn internal_response(message: &'static str) -> Response {
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
}

#[cfg(test)]
mod tests;
