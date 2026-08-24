//! Google OAuth2 and OpenID Connect provider struct and token decoding helpers.

use crate::client::HttpClientExt;
use crate::providers::google::types::GoogleTokenResponse;
use crate::user::ConnectUser;
use serde_json::Value;

/// Google OAuth2 & OpenID Connect provider implementation with JWKS validation and PKCE support.
pub struct GoogleProvider {
    pub(crate) client_id: String,
    pub(crate) client_secret: secrecy::SecretString,
    pub(crate) redirect_url: String,
    pub(crate) http_client: ::std::sync::Arc<dyn crate::client::HttpClient>,
    pub(crate) scopes: String,
    pub(crate) state: Option<String>,
    pub(crate) pkce_challenge: Option<String>,
    pub(crate) credential_mode: crate::configuration::CredentialMode,
    pub(crate) jwks_cache: crate::provider::JwksCache,
}

impl GoogleProvider {
    /// Creates a validated provider. Placeholder credentials select an offline mock.
    pub fn try_new(
        client_id: impl Into<String>,
        client_secret: secrecy::SecretString,
        redirect_url: impl Into<String>,
    ) -> Result<Self, crate::error::ConnectError> {
        let client_id = client_id.into();
        let redirect_url = redirect_url.into();
        crate::configuration::validate_redirect_url(&redirect_url)?;
        let credential_mode = crate::configuration::credential_mode(&client_id, &client_secret);
        let http_client =
            crate::configuration::provider_http_client(credential_mode, "google", None);

        Ok(Self {
            client_id,
            client_secret,
            redirect_url,
            http_client,
            scopes: "openid profile email".to_string(),
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
        client_secret: secrecy::SecretString,
        redirect_url: impl Into<String>,
    ) -> Self {
        let client_id = client_id.into();
        let mut redirect_url = redirect_url.into();
        let (credential_mode, invalid_reason) =
            match crate::configuration::validate_redirect_url(&redirect_url) {
                Ok(_) => (
                    crate::configuration::credential_mode(&client_id, &client_secret),
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
            crate::configuration::provider_http_client(credential_mode, "google", invalid_reason);

        Self {
            client_id,
            client_secret,
            redirect_url,
            http_client,
            scopes: "openid profile email".to_string(),
            state: None,
            pkce_challenge: None,
            credential_mode,
            jwks_cache: crate::provider::JwksCache::default(),
        }
    }

    /// Returns the selected credential mode.
    pub fn credential_mode(&self) -> crate::configuration::CredentialMode {
        self.credential_mode
    }

    /// Appends custom OAuth permission scopes to the Google login request.
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

    /// Configures retry attempts with exponential backoff on HTTP network errors.
    #[cfg(feature = "retry")]
    #[cfg_attr(mutants, mutants::skip)]
    pub fn with_retry(mut self, max_retries: u32) -> Self {
        if matches!(
            self.credential_mode,
            crate::configuration::CredentialMode::Live
        ) {
            self.http_client =
                ::std::sync::Arc::new(crate::client::ReqwestClient::new_with_retry(max_retries));
        }
        self
    }

    /// Overrides JWKS freshness bounds for this provider instance.
    pub fn with_jwks_cache_policy(mut self, policy: crate::provider::JwksCachePolicy) -> Self {
        self.jwks_cache = crate::provider::JwksCache::new(policy);
        self
    }

    pub(crate) async fn get_jwks_for_kid(
        &self,
        kid: &str,
    ) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, crate::error::ConnectError> {
        self.jwks_cache
            .get_for_kid(
                "https://www.googleapis.com/oauth2/v3/certs",
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
        // Exchange code for token
        let token_res = self
            .http_client
            .post("https://oauth2.googleapis.com/token")
            .form(form_data)
            .send()
            .await?
            .error_for_status()?
            .json::<GoogleTokenResponse>()
            .await?;

        let access_token = token_res.access_token;

        let mut user = if let Some(id_token) = &token_res.id_token {
            // Secure OIDC: Verify the signature of Google's id_token
            let header = jsonwebtoken::decode_header(id_token).map_err(|e| {
                crate::error::ConnectError::Provider(format!(
                    "Failed to decode Google id_token header: {}",
                    e
                ))
            })?;

            if let Some(kid) = header.kid.as_ref() {
                let jwks = self.get_jwks_for_kid(kid).await?;
                let jwk = jwks.find(kid).ok_or_else(|| {
                    crate::error::ConnectError::Provider(format!(
                        "Google JWK with key ID '{}' not found",
                        kid
                    ))
                })?;
                let decoding_key = jsonwebtoken::DecodingKey::from_jwk(jwk).map_err(|e| {
                    crate::error::ConnectError::Provider(format!(
                        "Failed to build Google decoding key: {}",
                        e
                    ))
                })?;

                let alg = match header.alg {
                    jsonwebtoken::Algorithm::RS256 => jsonwebtoken::Algorithm::RS256,
                    _ => {
                        return Err(crate::error::ConnectError::Provider(
                            "Unsupported algorithm in id_token header".to_string(),
                        ));
                    }
                };
                let mut validation = jsonwebtoken::Validation::new(alg);
                validation.set_audience(&[&self.client_id]);
                validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
                validation.validate_exp = true;
                if expected_nonce.is_some() {
                    validation.set_required_spec_claims(&["nonce"]);
                }

                let token_data =
                    jsonwebtoken::decode::<Value>(id_token, &decoding_key, &validation).map_err(
                        |e| {
                            crate::error::ConnectError::Provider(format!(
                                "Google id_token validation failed: {}",
                                e
                            ))
                        },
                    )?;

                let p = token_data.claims;

                if let Some(nonce) = expected_nonce {
                    let token_nonce = p["nonce"].as_str().unwrap_or("");
                    if !crate::provider::verify_nonce(token_nonce, nonce) {
                        return Err(crate::error::ConnectError::Provider(
                            "Google id_token nonce mismatch".to_owned(),
                        ));
                    }
                }

                ConnectUser {
                    id: p["sub"].as_str().map(String::from).ok_or_else(|| {
                        crate::error::ConnectError::Provider(
                            "Missing sub claim in Google id_token".to_owned(),
                        )
                    })?,
                    name: p["name"].as_str().map(String::from).unwrap_or_default(),
                    email: p["email"].as_str().map(String::from),
                    avatar_url: p["picture"]
                        .as_str()
                        .map(|s: &str| s.replace("=s96-c", "=s400-c")),
                    email_verified: p["email_verified"].as_bool(),
                    raw_data: p,
                    access_token: access_token.into(),
                    refresh_token: None,
                    expires_in: None,
                }
            } else {
                return Err(crate::error::ConnectError::Provider(
                    "Missing 'kid' header in Google id_token".to_owned(),
                ));
            }
        } else {
            use crate::provider::Provider;
            self.get_user_from_token(&access_token).await?
        };

        user.refresh_token = token_res.refresh_token.map(secrecy::SecretString::from);
        user.expires_in = token_res.expires_in;
        Ok(user)
    }
}
