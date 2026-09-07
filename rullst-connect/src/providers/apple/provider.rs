//! Apple OAuth2 provider struct, builders, and JWT verification helpers.

use crate::client::HttpClientExt;
use crate::providers::apple::types::AppleClaims;
use crate::user::ConnectUser;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// Apple OAuth2 provider implementation with Sign in with Apple (.p8 client_secret generator and OIDC JWKS token verification).
pub struct AppleProvider {
    pub(crate) client_id: String,
    pub(crate) team_id: String,
    pub(crate) key_id: String,
    pub(crate) private_key_pem: String,
    pub(crate) redirect_url: String,
    pub(crate) http_client: ::std::sync::Arc<dyn crate::client::HttpClient>,
    pub(crate) scopes: String,
    pub(crate) state: Option<String>,
    pub(crate) pkce_challenge: Option<String>,
    pub(crate) credential_mode: crate::configuration::CredentialMode,
    pub(crate) jwks_cache: crate::provider::JwksCache,
}

impl AppleProvider {
    /// Apple requires a Team ID, a Key ID, and the contents of a .p8 Private Key file
    /// to dynamically generate the client_secret JWT on every login.
    pub fn try_new(
        client_id: impl Into<String>,
        team_id: impl Into<String>,
        key_id: impl Into<String>,
        private_key_pem: impl Into<String>,
        redirect_url: impl Into<String>,
    ) -> Result<Self, crate::error::ConnectError> {
        let client_id = client_id.into();
        let team_id = team_id.into();
        let key_id = key_id.into();
        let private_key_pem = private_key_pem.into();
        let redirect_url = redirect_url.into();
        crate::configuration::validate_redirect_url(&redirect_url)?;
        let credential_mode = crate::configuration::credential_mode_for_values(&[
            &client_id,
            &team_id,
            &key_id,
            &private_key_pem,
        ]);
        let http_client =
            crate::configuration::provider_http_client(credential_mode, "apple", None);

        Ok(Self {
            client_id,
            team_id,
            key_id,
            private_key_pem,
            redirect_url,
            http_client,
            scopes: "name email".to_string(),
            state: None,
            pkce_challenge: None,
            credential_mode,
            jwks_cache: crate::provider::JwksCache::default(),
        })
    }

    /// Deprecated infallible constructor. Invalid configuration is disabled.
    #[cfg_attr(
        not(test),
        deprecated(since = "12.0.0", note = "use try_new and handle ConnectError")
    )]
    pub fn new(
        client_id: impl Into<String>,
        team_id: impl Into<String>,
        key_id: impl Into<String>,
        private_key_pem: impl Into<String>,
        redirect_url: impl Into<String>,
    ) -> Self {
        let client_id = client_id.into();
        let team_id = team_id.into();
        let key_id = key_id.into();
        let private_key_pem = private_key_pem.into();
        let mut redirect_url = redirect_url.into();
        let (credential_mode, invalid_reason) =
            match crate::configuration::validate_redirect_url(&redirect_url) {
                Ok(_) => (
                    crate::configuration::credential_mode_for_values(&[
                        &client_id,
                        &team_id,
                        &key_id,
                        &private_key_pem,
                    ]),
                    None,
                ),
                Err(error) => {
                    redirect_url = "about:blank".to_string();
                    (
                        crate::configuration::CredentialMode::Invalid,
                        Some(error.to_string()),
                    )
                }
            };
        let http_client =
            crate::configuration::provider_http_client(credential_mode, "apple", invalid_reason);
        Self {
            client_id,
            team_id,
            key_id,
            private_key_pem,
            redirect_url,
            http_client,
            scopes: "name email".to_string(),
            state: None,
            pkce_challenge: None,
            credential_mode,
            jwks_cache: crate::provider::JwksCache::default(),
        }
    }

    pub fn credential_mode(&self) -> crate::configuration::CredentialMode {
        self.credential_mode
    }

    /// Appends custom OAuth permission scopes to the Apple login request.
    pub fn with_scopes(mut self, scopes: &[&str]) -> Self {
        self.scopes = scopes.join(" ");
        self
    }

    /// Attaches an OAuth state parameter for CSRF mitigation.
    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Attaches a PKCE code challenge to the authorization request.
    pub fn with_pkce(mut self, challenge: impl Into<String>) -> Self {
        self.pkce_challenge = Some(challenge.into());
        self
    }

    /// Configures a custom HTTP client for live credentials.
    /// Mock and invalid credentials retain their network-free/fail-closed transport.
    pub fn with_http_client(
        mut self,
        client: ::std::sync::Arc<dyn crate::client::HttpClient>,
    ) -> Self {
        if matches!(
            self.credential_mode,
            crate::configuration::CredentialMode::Live
        ) {
            self.http_client = client;
        }
        self
    }

    pub fn with_jwks_cache_policy(mut self, policy: crate::provider::JwksCachePolicy) -> Self {
        self.jwks_cache = crate::provider::JwksCache::new(policy);
        self
    }

    pub(crate) fn generate_client_secret(&self) -> Result<String, crate::error::ConnectError> {
        if self.credential_mode.is_mock() {
            return Ok("mock_apple_client_secret".to_string());
        }
        if self.credential_mode.is_invalid() {
            return Err(crate::error::ConnectError::Offline(
                "Apple provider is disabled by invalid configuration".to_string(),
            ));
        }
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let claims = AppleClaims {
            iss: &self.team_id,
            iat: now,
            exp: now + 300, // 5 minutes expiration (short-lived credential)
            aud: "https://appleid.apple.com",
            sub: &self.client_id,
        };

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());

        let encoding_key = EncodingKey::from_ec_pem(self.private_key_pem.as_bytes())?;
        let token = encode(&header, &claims, &encoding_key)?;

        Ok(token)
    }

    pub(crate) async fn get_jwks_for_kid(
        &self,
        kid: &str,
    ) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, crate::error::ConnectError> {
        self.jwks_cache
            .get_for_kid(
                "https://appleid.apple.com/auth/keys",
                kid,
                self.http_client.as_ref(),
            )
            .await
    }

    pub(crate) async fn get_user_from_form(
        &self,
        form_data: &crate::provider::TokenExchangeForm<'_>,
        expected_nonce: Option<&str>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let token_res = self
            .http_client
            .post("https://appleid.apple.com/auth/token")
            .form(form_data)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        // Apple returns user data inside an "id_token" (JWT)
        let id_token_str = token_res["id_token"].as_str().ok_or_else(|| {
            crate::error::ConnectError::Token("Failed to get id_token from Apple".to_string())
        })?;
        let access_token = token_res["access_token"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                crate::error::ConnectError::Token(
                    "Failed to get access_token from Apple".to_string(),
                )
            })?;

        let mut user = self
            .decode_apple_id_token(id_token_str, expected_nonce)
            .await?;
        user.access_token = access_token.into();
        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = crate::provider::token_lifetime(&token_res)?;
        Ok(user)
    }

    pub(crate) async fn decode_apple_id_token(
        &self,
        id_token_str: &str,
        expected_nonce: Option<&str>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        if self.credential_mode.is_mock() {
            return self.mock_user(id_token_str);
        }

        let header = jsonwebtoken::decode_header(id_token_str).map_err(|error| {
            crate::error::ConnectError::Provider(format!(
                "Failed to decode Apple id_token header: {error}"
            ))
        })?;
        let kid = header.kid.as_deref().ok_or_else(|| {
            crate::error::ConnectError::Provider(
                "Missing 'kid' header in Apple id_token".to_string(),
            )
        })?;
        if header.alg != jsonwebtoken::Algorithm::RS256 {
            return Err(crate::error::ConnectError::Provider(
                "Unsupported algorithm in Apple id_token header".to_string(),
            ));
        }
        let jwks = self.get_jwks_for_kid(kid).await?;
        let jwk = jwks
            .find(kid)
            .ok_or_else(|| crate::error::ConnectError::JwkNotFound(kid.to_string()))?;
        let decoding_key = jsonwebtoken::DecodingKey::from_jwk(jwk)?;
        let validation = crate::provider::id_token::validation(
            jsonwebtoken::Algorithm::RS256,
            &self.client_id,
            &["https://appleid.apple.com"],
        );
        let payload = jsonwebtoken::decode::<Value>(id_token_str, &decoding_key, &validation)
            .map_err(|error| {
                crate::error::ConnectError::Provider(format!(
                    "Apple id_token signature or claims validation failed: {error}"
                ))
            })?
            .claims;

        crate::provider::id_token::validate_claims(&payload, &self.client_id, expected_nonce)?;

        Ok(ConnectUser {
            id: payload["sub"].as_str().map(String::from).ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing sub in Apple id_token".to_string())
            })?,
            name: String::with_capacity(256), // Developer needs to extract this from the form_post on first login
            email: payload["email"].as_str().map(String::from),
            avatar_url: None, // Apple does not provide avatars
            email_verified: None,
            raw_data: payload,
            access_token: id_token_str.to_string().into(),
            refresh_token: None,
            expires_in: None,
        })
    }

    pub(crate) fn mock_user(&self, token: &str) -> Result<ConnectUser, crate::error::ConnectError> {
        if !cfg!(any(test, feature = "mock")) {
            return Err(crate::error::ConnectError::Offline(
                "mock credentials are network-free but functional mock identities require the 'mock' feature"
                    .to_string(),
            ));
        }
        Ok(ConnectUser {
            id: "mock-user".to_string(),
            name: "Rullst Mock User".to_string(),
            email: Some("mock@example.invalid".to_string()),
            avatar_url: None,
            email_verified: Some(true),
            raw_data: serde_json::json!({ "provider": "apple", "mock": true }),
            access_token: token.to_string().into(),
            refresh_token: Some("mock_refresh_token".to_string().into()),
            expires_in: Some(3600),
        })
    }
}
