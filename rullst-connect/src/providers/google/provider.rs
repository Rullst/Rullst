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
    pub(crate) authorized_presenters: Vec<String>,
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
            authorized_presenters: Vec::new(),
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
            authorized_presenters: Vec::new(),
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

    /// Trusts native Google Sign-In clients as authorized presenters (`azp`)
    /// in [`Self::verify_id_token`].
    ///
    /// Android Credential Manager and iOS Google Sign-In request an ID token
    /// for the server (web) client ID: `aud` is this provider's `client_id`
    /// and `azp` is the Android or iOS client ID. List those native client
    /// IDs here. `aud` must still be exactly this `client_id`, the list never
    /// adds an audience, and the authorization-code flow keeps requiring
    /// `azp` (when present) to equal this `client_id`.
    ///
    /// At most 16 distinct IDs of 1–255 visible ASCII characters are accepted;
    /// calling this again replaces the list. Unpublished v13 API.
    ///
    /// # Errors
    /// Returns [`crate::error::ConnectError::InvalidConfiguration`] for an
    /// empty, oversized or non-printable ID or more than 16 IDs.
    pub fn try_with_authorized_presenters<I, S>(
        mut self,
        presenters: I,
    ) -> Result<Self, crate::error::ConnectError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.authorized_presenters =
            crate::provider::id_token::validate_authorized_presenters(presenters)?;
        Ok(self)
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
            let claims = self
                .verify_id_token_claims(id_token, &[], expected_nonce)
                .await?;
            user_from_id_token_claims(claims, access_token.into())?
        } else {
            if expected_nonce.is_some() && !self.credential_mode.is_mock() {
                return Err(crate::error::ConnectError::Provider(
                    "OIDC nonce-bound login requires an id_token".into(),
                ));
            }
            use crate::provider::Provider;
            self.get_user_from_token(&access_token).await?
        };

        user.refresh_token = token_res.refresh_token.map(secrecy::SecretString::from);
        user.expires_in = crate::provider::validate_token_lifetime(token_res.expires_in)?;
        Ok(user)
    }

    /// Verifies a Google ID token issued to this application and returns its user.
    ///
    /// Use this to sign in a user whose native or mobile client obtained an ID
    /// token for this `client_id` (for example with Google Sign-In) and sent it
    /// to your server. Unlike [`crate::provider::Provider::get_user_from_token`],
    /// the result is bound to this application: the RS256 signature is verified
    /// through Google's rotating JWKS, `iss` must be Google, `aud` exactly this
    /// `client_id`, `azp` (when present) this `client_id` or a native client
    /// ID configured with [`Self::try_with_authorized_presenters`], and `exp`,
    /// `iat` and `nonce` must be valid. Android and iOS Google Sign-In tokens
    /// requested for this server client carry the native client ID in `azp`,
    /// so configure those IDs before accepting them. Generate `expected_nonce` on the server for this
    /// sign-in attempt, let the client pass it to Google, and consume it once.
    ///
    /// The returned user carries the verified ID token in `access_token`; this
    /// flow yields no provider access or refresh token. The provider's userinfo
    /// endpoint is not called. Mock credentials return a deterministic offline
    /// identity when the `mock` feature is enabled.
    ///
    /// ```rust,no_run
    /// use rullst_connect::prelude::{ConnectError, ConnectUser, GoogleProvider};
    ///
    /// async fn sign_in_native_google_user(
    ///     google: &GoogleProvider,
    ///     id_token: &str,
    ///     nonce_issued_for_this_attempt: &str,
    /// ) -> Result<ConnectUser, ConnectError> {
    ///     google
    ///         .verify_id_token(id_token, nonce_issued_for_this_attempt)
    ///         .await
    /// }
    /// ```
    pub async fn verify_id_token(
        &self,
        id_token: &str,
        expected_nonce: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        crate::provider::id_token::validate_verification_input(id_token, expected_nonce)?;
        if self.credential_mode.is_mock() {
            return crate::provider::id_token::mock_verified_user("google", id_token);
        }
        let claims = self
            .verify_id_token_claims(id_token, &self.authorized_presenters, Some(expected_nonce))
            .await?;
        user_from_id_token_claims(claims, secrecy::SecretString::from(id_token.to_owned()))
    }

    /// Verifies a Google ID token's RS256 signature through the rotating JWKS,
    /// then its issuer, audience (this `client_id`), `azp`, lifetime and,
    /// when supplied, nonce. Returns the verified claims.
    pub(crate) async fn verify_id_token_claims(
        &self,
        id_token: &str,
        presenters: &[String],
        expected_nonce: Option<&str>,
    ) -> Result<Value, crate::error::ConnectError> {
        let header = jsonwebtoken::decode_header(id_token).map_err(|e| {
            crate::error::ConnectError::Provider(format!(
                "Failed to decode Google id_token header: {}",
                e
            ))
        })?;
        let kid = header.kid.as_ref().ok_or_else(|| {
            crate::error::ConnectError::Provider(
                "Missing 'kid' header in Google id_token".to_owned(),
            )
        })?;
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
        let validation = crate::provider::id_token::validation(
            alg,
            &self.client_id,
            &["https://accounts.google.com", "accounts.google.com"],
        );

        let token_data = jsonwebtoken::decode::<Value>(id_token, &decoding_key, &validation)
            .map_err(|e| {
                crate::error::ConnectError::Provider(format!(
                    "Google id_token validation failed: {}",
                    e
                ))
            })?;

        let claims = token_data.claims;
        crate::provider::id_token::validate_claims_with_presenters(
            &claims,
            &self.client_id,
            presenters,
            expected_nonce,
        )?;
        Ok(claims)
    }
}

fn user_from_id_token_claims(
    p: Value,
    access_token: secrecy::SecretString,
) -> Result<ConnectUser, crate::error::ConnectError> {
    Ok(ConnectUser {
        id: p["sub"].as_str().map(String::from).ok_or_else(|| {
            crate::error::ConnectError::Provider("Missing sub claim in Google id_token".to_owned())
        })?,
        name: p["name"].as_str().map(String::from).unwrap_or_default(),
        email: p["email"].as_str().map(String::from),
        avatar_url: p["picture"]
            .as_str()
            .map(|s: &str| s.replace("=s96-c", "=s400-c")),
        email_verified: p["email_verified"].as_bool(),
        raw_data: p,
        access_token,
        refresh_token: None,
        expires_in: None,
    })
}
