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
}

impl GoogleProvider {
    /// Creates a new `GoogleProvider` with strict client credential validation.
    pub fn new(
        client_id: String,
        client_secret: secrecy::SecretString,
        redirect_url: String,
    ) -> Self {
        assert!(
            !client_id.is_empty(),
            "Socialite Error: client_id cannot be empty"
        );
        assert!(
            !secrecy::ExposeSecret::expose_secret(&client_secret).is_empty(),
            "Socialite Error: client_secret cannot be empty"
        );
        assert!(
            redirect_url.starts_with("http"),
            "Socialite Error: redirect_url must be a valid HTTP/HTTPS URL"
        );

        Self {
            client_id,
            client_secret,
            redirect_url,
            http_client: crate::client::DEFAULT_HTTP_CLIENT.clone(),
            scopes: "openid profile email".to_string(),
            state: None,
            pkce_challenge: None,
        }
    }

    /// Appends custom OAuth permission scopes to the Google login request.
    pub fn with_scopes(mut self, scopes: &[&str]) -> Self {
        self.scopes = scopes.join(" ");
        self
    }

    /// Attaches an OAuth state parameter for CSRF mitigation.
    pub fn with_state(mut self, state: &str) -> Self {
        self.state = Some(state.to_owned());
        self
    }

    /// Attaches a PKCE code challenge to the authorization request.
    pub fn with_pkce(mut self, challenge: &str) -> Self {
        self.pkce_challenge = Some(challenge.to_owned());
        self
    }

    /// Configures a custom HTTP client (e.g. for mock testing or specialized connection pooling).
    pub fn with_http_client(
        mut self,
        client: ::std::sync::Arc<dyn crate::client::HttpClient>,
    ) -> Self {
        self.http_client = client;
        self
    }

    /// Configures retry attempts with exponential backoff on HTTP network errors.
    #[cfg(feature = "retry")]
    #[cfg_attr(mutants, mutants::skip)]
    pub fn with_retry(mut self, max_retries: u32) -> Self {
        self.http_client =
            ::std::sync::Arc::new(crate::client::ReqwestClient::new_with_retry(max_retries));
        self
    }

    pub(crate) async fn get_jwks(
        &self,
    ) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, crate::error::ConnectError> {
        crate::provider::fetch_and_cache_jwks(
            "https://www.googleapis.com/oauth2/v3/certs",
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
                let jwks = self.get_jwks().await?;
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
