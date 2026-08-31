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

/// Parameters to exchange the authorization code for tokens.
#[derive(Debug, Default, Clone)]
pub struct ExchangeParams<'a> {
    /// The authorization code received from the authorization server callback.
    pub auth_code: &'a str,
    /// PKCE code verifier (RFC 7636).
    pub code_verifier: Option<&'a str>,
    /// Expected nonce for OpenID Connect ID token verification.
    pub expected_nonce: Option<&'a str>,
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
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Oauth2TokenResponse {
    /// Bearer access token string.
    pub access_token: String,
    /// Optional refresh token string.
    pub refresh_token: Option<String>,
    /// Token lifetime in seconds.
    pub expires_in: Option<u64>,
}

/// Constant-time cryptographic verification of OpenID Connect nonces.
pub(crate) fn verify_nonce(token_nonce: &str, expected_nonce: &str) -> bool {
    use sha2::{Digest, Sha256};
    use subtle::ConstantTimeEq;

    let hash_token = Sha256::digest(token_nonce.as_bytes());
    let hash_expected = Sha256::digest(expected_nonce.as_bytes());

    bool::from(hash_token.ct_eq(&hash_expected))
}
