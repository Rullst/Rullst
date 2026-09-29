//! Form-POST variant of the managed callback for `response_mode=form_post` providers.

use super::{AuthSession, consume_challenge, invalid_callback, request_session};
use crate::error::ConnectError;
use crate::extractors::AuthCallback;
use crate::provider::ExchangeParams;
use axum::extract::{FromRequest, Request};
use axum::http::{Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::fmt;

/// Largest accepted callback body. Sign in with Apple posts `code`, `state`,
/// `id_token` and, on first sign-in, a small `user` JSON value.
const MAX_FORM_CALLBACK_BYTES: usize = 16 * 1024;

/// Validated `application/x-www-form-urlencoded` callback plus the consumed
/// OIDC nonce and PKCE verifier.
///
/// Use this extractor for providers that return the authorization response in
/// a POST body (`response_mode=form_post`), such as Sign in with Apple. It
/// consumes the same challenge that [`super::begin_oidc_session`] or
/// [`super::begin_oauth_session`] stored and applies the same expiry,
/// constant-time state comparison and single use as [`AuthSession`].
///
/// The request must be a `POST` with an `application/x-www-form-urlencoded`
/// body of at most 16 KiB; otherwise it is rejected before the challenge is
/// touched. Unknown fields such as Apple's `id_token` and `user` are ignored.
///
/// The provider's POST is cross-site, so the session cookie that identifies the
/// stored challenge must be `SameSite=None; Secure`. Use a dedicated session
/// layer for the provider's start and callback routes instead of relaxing the
/// application's authenticated session cookie.
///
/// ```rust,no_run
/// use rullst_connect::prelude::*;
///
/// async fn apple_callback(
///     callback: AuthSessionForm,
///     apple: &AppleProvider,
/// ) -> Result<ConnectUser, ConnectError> {
///     apple.get_user(callback.exchange_params()?).await
/// }
/// ```
#[derive(Clone)]
pub struct AuthSessionForm {
    session: AuthSession,
}

impl AuthSessionForm {
    /// Builds the exact parameters for the provider token and ID-token exchange.
    pub fn exchange_params(&self) -> Result<ExchangeParams<'_>, ConnectError> {
        self.session.exchange_params()
    }

    /// Returns the validated callback in its query-flow representation.
    pub fn auth_session(&self) -> &AuthSession {
        &self.session
    }

    /// Converts this extractor into the validated [`AuthSession`].
    pub fn into_auth_session(self) -> AuthSession {
        self.session
    }
}

impl fmt::Debug for AuthSessionForm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthSessionForm")
            .field("session", &self.session)
            .finish()
    }
}

impl<S> FromRequest<S> for AuthSessionForm
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = request.into_parts();
        if parts.method != Method::POST {
            return Err((
                StatusCode::METHOD_NOT_ALLOWED,
                "OAuth form callback requires POST",
            )
                .into_response());
        }
        if !is_form_urlencoded(&parts.headers) {
            return Err((
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "OAuth form callback requires application/x-www-form-urlencoded",
            )
                .into_response());
        }
        let session = request_session(&parts).map_err(IntoResponse::into_response)?;
        let body = axum::body::to_bytes(body, MAX_FORM_CALLBACK_BYTES)
            .await
            .map_err(|_| {
                (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "OAuth form callback body is too large or unreadable",
                )
                    .into_response()
            })?;
        let callback: AuthCallback = serde_urlencoded::from_bytes(&body)
            .map_err(|_| invalid_callback("Malformed OAuth form callback").into_response())?;
        let session = consume_challenge(&session, callback)
            .await
            .map_err(IntoResponse::into_response)?;
        Ok(Self { session })
    }
}

fn is_form_urlencoded(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media_type| {
            media_type
                .trim()
                .eq_ignore_ascii_case("application/x-www-form-urlencoded")
        })
}

#[cfg(test)]
mod tests;
