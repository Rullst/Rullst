use async_trait::async_trait;
use serde_json::Value;

use crate::client::HttpClientExt;
use crate::error::ConnectError;
use crate::provider::Provider;
use crate::user::ConnectUser;

use super::discovery::OidcProvider;

impl OidcProvider {
    pub(crate) async fn get_jwks_for_kid(
        &self,
        kid: &str,
    ) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, ConnectError> {
        self.jwks_cache
            .get_for_kid(&self.jwks_uri, kid, self.http_client.as_ref())
            .await
    }

    #[tracing::instrument(skip_all, fields(has_nonce = expected_nonce.is_some()))]
    pub(crate) async fn get_user_from_form(
        &self,
        form_data: &(impl serde::Serialize + Sync),
        expected_nonce: Option<&str>,
    ) -> Result<ConnectUser, ConnectError> {
        let token_res = self.post_token_form(form_data).await?;
        self.user_from_token_response(&token_res, expected_nonce)
            .await
    }

    async fn post_token_form(
        &self,
        form_data: &(impl serde::Serialize + Sync),
    ) -> Result<Value, ConnectError> {
        self.client_authentication
            .authorize(
                self.http_client.post(self.token_url()),
                &self.client_id,
                secrecy::ExposeSecret::expose_secret(&self.client_secret),
            )
            .form(form_data)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await
    }

    async fn user_from_token_response(
        &self,
        token_res: &Value,
        expected_nonce: Option<&str>,
    ) -> Result<ConnectUser, ConnectError> {
        let access_token = token_res["access_token"]
            .as_str()
            .ok_or_else(|| ConnectError::Token("Failed to get access_token".to_string()))?;

        let mut user = if let Some(id_token) = token_res["id_token"].as_str() {
            let claims = self
                .verify_id_token_claims(id_token, expected_nonce)
                .await?;
            user_from_id_token_claims(claims, secrecy::SecretString::from(access_token.to_owned()))?
        } else {
            if expected_nonce.is_some() && !self.credential_mode.is_mock() {
                return Err(ConnectError::Provider(
                    "OIDC nonce-bound login requires an id_token".into(),
                ));
            }
            use crate::provider::Provider;
            self.get_user_from_token(access_token).await?
        };

        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = crate::provider::token_lifetime(token_res)?;

        Ok(user)
    }

    /// Verifies an ID token issued to this application and returns its user.
    ///
    /// Use this to sign in a user whose native or mobile client obtained an ID
    /// token from this issuer for this `client_id` and sent it to your server.
    /// Unlike [`Provider::get_user_from_token`], the result is bound to this
    /// application: the signature is verified through the discovered, rotating
    /// JWKS (RS256/384/512, ES256/384 or EdDSA), `iss` must equal the
    /// discovered issuer exactly, `aud` exactly this `client_id`, `azp` (when
    /// present) this `client_id`, and `exp`, `iat` and `nonce` must be valid.
    /// Generate `expected_nonce` on the server for this sign-in attempt, let
    /// the client pass it to the provider, and consume it once.
    ///
    /// The returned user carries the verified ID token in `access_token`; this
    /// flow yields no provider access or refresh token. The userinfo endpoint
    /// is not called, and the token must contain a `name` claim. Mock
    /// credentials return a deterministic offline identity when the `mock`
    /// feature is enabled.
    pub async fn verify_id_token(
        &self,
        id_token: &str,
        expected_nonce: &str,
    ) -> Result<ConnectUser, ConnectError> {
        crate::provider::id_token::validate_verification_input(id_token, expected_nonce)?;
        if self.credential_mode.is_mock() {
            return crate::provider::id_token::mock_verified_user("oidc", id_token);
        }
        let claims = self
            .verify_id_token_claims(id_token, Some(expected_nonce))
            .await?;
        user_from_id_token_claims(claims, secrecy::SecretString::from(id_token.to_owned()))
    }

    /// Verifies an ID token's signature through the discovered JWKS, then its
    /// exact issuer, audience, `azp`, lifetime and, when supplied, nonce.
    pub(crate) async fn verify_id_token_claims(
        &self,
        id_token: &str,
        expected_nonce: Option<&str>,
    ) -> Result<Value, ConnectError> {
        let header = jsonwebtoken::decode_header(id_token).map_err(|e| {
            ConnectError::Provider(format!("Failed to decode OIDC id_token header: {}", e))
        })?;
        let kid = header.kid.as_ref().ok_or_else(|| {
            ConnectError::Provider("Missing 'kid' header in OIDC id_token".to_owned())
        })?;
        let jwks = self.get_jwks_for_kid(kid).await?;
        let jwk = jwks.find(kid).ok_or_else(|| {
            ConnectError::Provider(format!("OIDC JWK with key ID '{}' not found", kid))
        })?;
        let decoding_key = jsonwebtoken::DecodingKey::from_jwk(jwk).map_err(|e| {
            ConnectError::Provider(format!("Failed to build OIDC decoding key from JWK: {}", e))
        })?;
        let alg = match header.alg {
            jsonwebtoken::Algorithm::RS256
            | jsonwebtoken::Algorithm::RS384
            | jsonwebtoken::Algorithm::RS512
            | jsonwebtoken::Algorithm::ES256
            | jsonwebtoken::Algorithm::ES384
            | jsonwebtoken::Algorithm::EdDSA => header.alg,
            _ => {
                return Err(ConnectError::Provider(
                    "OIDC token header specifies an insecure or symmetric algorithm".to_string(),
                ));
            }
        };
        let validation =
            crate::provider::id_token::validation(alg, &self.client_id, &[&self.issuer]);

        let claims = jsonwebtoken::decode::<Value>(id_token, &decoding_key, &validation)
            .map_err(|e| {
                ConnectError::Provider(format!(
                    "OIDC id_token signature or claims validation failed: {}",
                    e
                ))
            })?
            .claims;
        crate::provider::id_token::validate_claims(&claims, &self.client_id, expected_nonce)?;
        Ok(claims)
    }
}

fn user_from_id_token_claims(
    payload: Value,
    access_token: secrecy::SecretString,
) -> Result<ConnectUser, ConnectError> {
    Ok(ConnectUser {
        id: payload["sub"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| ConnectError::Provider("Missing sub in id_token".to_owned()))?,
        name: payload["name"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| ConnectError::Provider("Missing name in id_token".to_owned()))?,
        email: payload["email"].as_str().map(String::from),
        avatar_url: payload["picture"].as_str().map(String::from),
        email_verified: payload["email_verified"].as_bool(),
        raw_data: payload,
        access_token,
        refresh_token: None,
        expires_in: None,
    })
}

#[async_trait]
impl Provider for OidcProvider {
    fn redirect_url(&self) -> String {
        if self.credential_mode.is_mock() {
            return crate::configuration::mock_redirect_url(
                "oidc",
                self.state.as_deref(),
                self.pkce_challenge.as_deref(),
            );
        }
        let mut params = crate::provider::build_oauth_params(
            &self.authorization_endpoint,
            &self.client_id,
            &self.redirect_url,
            &self.scopes,
            self.state.as_deref(),
            self.pkce_challenge.as_deref(),
        );
        params.append_pair("response_type", "code");
        params.finish()
    }

    #[tracing::instrument(skip(self, params))]
    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, ConnectError> {
        let form_data = crate::provider::TokenExchangeForm {
            client_id: self.client_id.as_str(),
            client_secret: self
                .client_authentication
                .body_secret(secrecy::ExposeSecret::expose_secret(&self.client_secret)),
            code: params.auth_code,
            grant_type: Some("authorization_code"),
            redirect_uri: self.redirect_url.as_str(),
            code_verifier: params.code_verifier,
        };
        self.get_user_from_form(&form_data, params.expected_nonce)
            .await
    }

    #[tracing::instrument(skip(self, access_token))]
    async fn get_user_from_token(&self, access_token: &str) -> Result<ConnectUser, ConnectError> {
        let user_res = self
            .http_client
            .get(&self.userinfo_endpoint)
            .bearer_auth(access_token)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        Ok(ConnectUser {
            id: user_res["sub"].as_str().map(String::from).ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing sub in userinfo".to_owned())
            })?,
            name: user_res["name"].as_str().map(String::from).ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing name in userinfo".to_owned())
            })?,
            email: user_res["email"].as_str().map(String::from),
            avatar_url: user_res["picture"].as_str().map(String::from),
            email_verified: user_res["email_verified"].as_bool(),
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_owned()),
            refresh_token: None,
            expires_in: None,
        })
    }

    fn token_url(&self) -> String {
        self.token_endpoint.clone()
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<ConnectUser, ConnectError> {
        let mut form_data = vec![("client_id", self.client_id.as_str())];
        if let Some(secret) = self
            .client_authentication
            .body_secret(secrecy::ExposeSecret::expose_secret(&self.client_secret))
        {
            form_data.push(("client_secret", secret));
        }
        form_data.push(("refresh_token", refresh_token));
        form_data.push(("grant_type", "refresh_token"));
        let token_res = self.post_token_form(&form_data).await?;
        self.user_from_token_response(&token_res, None)
            .await
            .map_err(|source| crate::error::refresh_incomplete(&token_res, source))
    }
}
