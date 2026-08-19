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
}

impl AppleProvider {
    /// Apple requires a Team ID, a Key ID, and the contents of a .p8 Private Key file
    /// to dynamically generate the client_secret JWT on every login.
    pub fn new(
        client_id: String,
        team_id: String,
        key_id: String,
        private_key_pem: String,
        redirect_url: String,
    ) -> Self {
        Self {
            client_id,
            team_id,
            key_id,
            private_key_pem,
            redirect_url,
            http_client: crate::client::DEFAULT_HTTP_CLIENT.clone(),
            scopes: "name email".to_string(),
            state: None,
            pkce_challenge: None,
        }
    }

    /// Appends custom OAuth permission scopes to the Apple login request.
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

    pub(crate) fn generate_client_secret(&self) -> Result<String, crate::error::ConnectError> {
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

    pub(crate) async fn get_jwks(
        &self,
    ) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, crate::error::ConnectError> {
        crate::provider::fetch_and_cache_jwks(
            "https://appleid.apple.com/auth/keys",
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
        user.expires_in = token_res["expires_in"]
            .as_u64()
            .or_else(|| token_res["expires_in"].as_i64().map(|v| v as u64));
        Ok(user)
    }

    pub(crate) async fn decode_apple_id_token(
        &self,
        id_token_str: &str,
        expected_nonce: Option<&str>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let mut payload: Option<Value> = None;

        if let Ok(header) = jsonwebtoken::decode_header(id_token_str)
            && let Some(kid) = header.kid.as_ref()
            && let Ok(jwks) = self.get_jwks().await
            && let Some(jwk) = jwks.find(kid)
            && let Ok(decoding_key) = jsonwebtoken::DecodingKey::from_jwk(jwk)
        {
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
            validation.set_issuer(&["https://appleid.apple.com"]);
            validation.validate_exp = true;
            if expected_nonce.is_some() {
                validation.set_required_spec_claims(&["nonce"]);
            }

            if let Ok(token_data) =
                jsonwebtoken::decode::<Value>(id_token_str, &decoding_key, &validation)
            {
                payload = Some(token_data.claims);
            }
        }

        let payload = match payload {
            Some(p) => p,
            None => {
                return Err(crate::error::ConnectError::Provider(
                    "Failed to verify Apple id_token signature or claims".to_string(),
                ));
            }
        };

        if let Some(nonce) = expected_nonce {
            let token_nonce = payload["nonce"].as_str().unwrap_or("");
            if !crate::provider::verify_nonce(token_nonce, nonce) {
                return Err(crate::error::ConnectError::Provider(
                    "Apple id_token nonce mismatch".to_owned(),
                ));
            }
        }

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
}
