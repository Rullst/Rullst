//! Core asynchronous Provider trait definition.

use crate::provider::RevocationTokenKind;
use crate::provider::types::ExchangeParams;
use crate::user::ConnectUser;
use async_trait::async_trait;

/// The core trait implemented by all OAuth2 providers in Rullst Connect.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Returns the authorization URL to redirect the user to the provider's login screen.
    fn redirect_url(&self) -> String;

    /// Returns the authorization URL with a `state` parameter appended.
    /// It is highly recommended to use this to prevent CSRF attacks.
    fn redirect_url_with_state(&self, state: &str) -> String {
        let mut string = self.redirect_url();
        // Pre-allocate capacity to prevent reallocation when appending query parameters
        string.reserve(8_usize.saturating_add(state.len()));
        let separator = if string.contains('?') { '&' } else { '?' };
        string.push(separator);
        let start_position = string.len();
        let mut serializer = url::form_urlencoded::Serializer::for_suffix(string, start_position);
        serializer.append_pair("state", state);
        serializer.finish()
    }

    /// Returns the authorization URL with a PKCE `code_challenge` appended.
    /// Useful for providers that enforce PKCE (like Twitter/X v2).
    fn redirect_url_with_pkce(&self, code_challenge: &str) -> String {
        let mut string = self.redirect_url();
        // Pre-allocate capacity to prevent reallocation when appending query parameters
        string.reserve(44_usize.saturating_add(code_challenge.len()));
        let separator = if string.contains('?') { '&' } else { '?' };
        string.push(separator);
        let start_position = string.len();
        let mut serializer = url::form_urlencoded::Serializer::for_suffix(string, start_position);
        serializer.append_pair("code_challenge", code_challenge);
        serializer.append_pair("code_challenge_method", "S256");
        serializer.finish()
    }

    /// Returns the authorization URL with a PKCE `code_challenge` and a `state` parameter appended.
    fn redirect_url_with_pkce_and_state(&self, code_challenge: &str, state: &str) -> String {
        let mut string = self.redirect_url();
        // Pre-allocate capacity to prevent reallocation when appending query parameters
        string.reserve(
            52_usize
                .saturating_add(code_challenge.len())
                .saturating_add(state.len()),
        );
        let separator = if string.contains('?') { '&' } else { '?' };
        string.push(separator);
        let start_position = string.len();
        let mut serializer = url::form_urlencoded::Serializer::for_suffix(string, start_position);
        serializer.append_pair("code_challenge", code_challenge);
        serializer.append_pair("code_challenge_method", "S256");
        serializer.append_pair("state", state);
        serializer.finish()
    }

    /// Exchanges the authorization code for an access token and fetches the user's profile.
    /// Returns a standardized `ConnectUser` or a `ConnectError`.
    async fn get_user(
        &self,
        params: ExchangeParams<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError>;

    /// Fetches the user's profile using an existing access token.
    /// This bypasses the authorization code exchange step.
    async fn get_user_from_token(
        &self,
        access_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError>;

    /// Returns the URL used to exchange the authorization code for an access token.
    fn token_url(&self) -> String;

    /// Exchanges a refresh token for a new access token and fetches the user profile.
    async fn refresh_token(
        &self,
        _refresh_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        Err(crate::error::ConnectError::Token(
            "Refresh token is not supported by this provider".to_string(),
        ))
    }

    /// Revokes an access token directly on the provider's authorization server.
    ///
    /// Providers that only support refresh-token revocation fail explicitly; use
    /// [`Self::revoke_refresh_token`] for that token category.
    async fn revoke_token(&self, token: &str) -> Result<(), crate::error::ConnectError> {
        self.revoke_token_with_kind(token, RevocationTokenKind::AccessToken)
            .await
    }

    /// Revokes a refresh token directly on the provider's authorization server.
    async fn revoke_refresh_token(&self, token: &str) -> Result<(), crate::error::ConnectError> {
        self.revoke_token_with_kind(token, RevocationTokenKind::RefreshToken)
            .await
    }

    /// Revokes a token with an explicit category for provider-specific protocol mapping.
    async fn revoke_token_with_kind(
        &self,
        _token: &str,
        _kind: RevocationTokenKind,
    ) -> Result<(), crate::error::ConnectError> {
        Err(crate::error::ConnectError::Token(
            "Token revocation is not supported by this provider".to_string(),
        ))
    }

    /// Initiates a device authorization flow (RFC 8628).
    /// Returns the device code, user code, and verification URI.
    async fn request_device_code(
        &self,
    ) -> Result<crate::user::DeviceAuthorizationResponse, crate::error::ConnectError> {
        Err(crate::error::ConnectError::Provider(
            "Device Authorization is not supported by this provider".into(),
        ))
    }

    /// Polls the provider for the access token during a device authorization flow.
    /// Returns the user's profile if the user has authorized the device.
    async fn poll_device_token(
        &self,
        _device_code: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        Err(crate::error::ConnectError::Provider(
            "Device Authorization is not supported by this provider".into(),
        ))
    }
}
