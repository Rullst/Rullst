//! Parameters, forms, and token response types for OAuth2 providers.

/// OAuth token category supplied to a provider revocation endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RevocationTokenKind {
    /// A bearer access token.
    AccessToken,
    /// A refresh token used to obtain new access tokens.
    RefreshToken,
}

impl RevocationTokenKind {
    /// Returns the RFC 7009 token type hint value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AccessToken => "access_token",
            Self::RefreshToken => "refresh_token",
        }
    }
}

/// Helper to construct standard OAuth2 parameters to reduce boilerplate.
pub fn build_oauth_params<'a>(
    base_url: &str,
    client_id: &'a str,
    redirect_uri: &'a str,
    scopes: &'a str,
    state: Option<&'a str>,
    pkce_challenge: Option<&'a str>,
) -> url::form_urlencoded::Serializer<'a, String> {
    let mut string = String::with_capacity(base_url.len() + 256);
    string.push_str(base_url);
    let separator = if base_url.contains('?') { '&' } else { '?' };
    string.push(separator);
    let start_position = string.len();
    let mut params = url::form_urlencoded::Serializer::for_suffix(string, start_position);
    params.append_pair("client_id", client_id);
    params.append_pair("redirect_uri", redirect_uri);
    if !scopes.is_empty() {
        params.append_pair("scope", scopes);
    }
    if let Some(s) = state {
        params.append_pair("state", s);
    }
    if let Some(p) = pkce_challenge {
        params.append_pair("code_challenge", p);
        params.append_pair("code_challenge_method", "S256");
    }
    params
}

const REDACTED: &str = "[REDACTED]";

/// Parameters to exchange the authorization code for tokens.
///
/// `Debug` output redacts the code, PKCE verifier and nonce.
#[derive(Default, Clone)]
pub struct ExchangeParams<'a> {
    /// The authorization code received from the authorization server callback.
    pub auth_code: &'a str,
    /// PKCE code verifier (RFC 7636).
    pub code_verifier: Option<&'a str>,
    /// Expected nonce for OpenID Connect ID token verification.
    pub expected_nonce: Option<&'a str>,
}

impl std::fmt::Debug for ExchangeParams<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExchangeParams")
            .field("auth_code", &REDACTED)
            .field("code_verifier", &self.code_verifier.map(|_| REDACTED))
            .field("expected_nonce", &self.expected_nonce.map(|_| REDACTED))
            .finish()
    }
}

/// Standard form payload for OAuth2 authorization code exchange.
#[derive(serde::Serialize)]
pub struct TokenExchangeForm<'a> {
    /// Client ID of the OAuth application.
    pub client_id: &'a str,
    /// Client Secret (if confidential client).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<&'a str>,
    /// Authorization code.
    pub code: &'a str,
    /// Grant type (usually `authorization_code`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_type: Option<&'a str>,
    /// Registered redirect URI.
    pub redirect_uri: &'a str,
    /// PKCE code verifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_verifier: Option<&'a str>,
}

/// The response containing token information from a standard OAuth2 exchange.
///
/// `Debug` output redacts the access and refresh tokens. The fields remain
/// plaintext `String`s, so callers must not log or persist them directly.
#[derive(Clone, serde::Deserialize)]
pub struct Oauth2TokenResponse {
    /// Bearer access token string.
    pub access_token: String,
    /// Optional refresh token string.
    pub refresh_token: Option<String>,
    /// Token lifetime in seconds.
    pub expires_in: Option<u64>,
}

impl std::fmt::Debug for Oauth2TokenResponse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Oauth2TokenResponse")
            .field("access_token", &REDACTED)
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| REDACTED),
            )
            .field("expires_in", &self.expires_in)
            .finish()
    }
}

/// Constant-time cryptographic verification of OpenID Connect nonces.
pub(crate) fn verify_nonce(token_nonce: &str, expected_nonce: &str) -> bool {
    use sha2::{Digest, Sha256};
    use subtle::ConstantTimeEq;

    let hash_token = Sha256::digest(token_nonce.as_bytes());
    let hash_expected = Sha256::digest(expected_nonce.as_bytes());

    bool::from(hash_token.ct_eq(&hash_expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_response_debug_redacts_access_and_refresh_tokens() {
        let response = Oauth2TokenResponse {
            access_token: "plaintext-access-token".to_string(),
            refresh_token: Some("plaintext-refresh-token".to_string()),
            expires_in: Some(3600),
        };
        let debug = format!("{response:?}");
        assert!(!debug.contains("plaintext-access-token"), "{debug}");
        assert!(!debug.contains("plaintext-refresh-token"), "{debug}");
        assert!(debug.contains("3600"), "non-secret lifetime stays visible");
        assert!(format!("{response:#?}").contains("Oauth2TokenResponse"));

        let without_refresh = Oauth2TokenResponse {
            refresh_token: None,
            ..response
        };
        assert!(!format!("{without_refresh:?}").contains("plaintext-access-token"));
    }

    #[test]
    fn exchange_params_debug_redacts_code_verifier_and_nonce() {
        let params = ExchangeParams {
            auth_code: "plaintext-authorization-code",
            code_verifier: Some("plaintext-pkce-verifier"),
            expected_nonce: Some("plaintext-oidc-nonce"),
        };
        let debug = format!("{params:?}");
        for secret in [
            "plaintext-authorization-code",
            "plaintext-pkce-verifier",
            "plaintext-oidc-nonce",
        ] {
            assert!(!debug.contains(secret), "{debug}");
        }
        assert!(format!("{:?}", ExchangeParams::default()).contains("ExchangeParams"));
    }
}
