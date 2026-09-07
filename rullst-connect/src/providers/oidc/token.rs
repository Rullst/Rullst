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

    #[tracing::instrument(skip(self, form_data))]
    pub(crate) async fn get_user_from_form(
        &self,
        form_data: &(impl serde::Serialize + Sync),
        expected_nonce: Option<&str>,
    ) -> Result<ConnectUser, ConnectError> {
        let token_res = self
            .http_client
            .post(self.token_url())
            .form(form_data)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let access_token = token_res["access_token"]
            .as_str()
            .ok_or_else(|| ConnectError::Token("Failed to get access_token".to_string()))?;

        let mut user = if let Some(id_token) = token_res["id_token"].as_str() {
            // Cryptographic OIDC Signature Validation
            let header = jsonwebtoken::decode_header(id_token).map_err(|e| {
                crate::error::ConnectError::Provider(format!(
                    "Failed to decode OIDC id_token header: {}",
                    e
                ))
            })?;

            if let Some(kid) = header.kid.as_ref() {
                let jwks = self.get_jwks_for_kid(kid).await?;
                let jwk = jwks.find(kid).ok_or_else(|| {
                    crate::error::ConnectError::Provider(format!(
                        "OIDC JWK with key ID '{}' not found",
                        kid
                    ))
                })?;
                let decoding_key = jsonwebtoken::DecodingKey::from_jwk(jwk).map_err(|e| {
                    crate::error::ConnectError::Provider(format!(
                        "Failed to build OIDC decoding key from JWK: {}",
                        e
                    ))
                })?;
                let alg = match header.alg {
                    jsonwebtoken::Algorithm::RS256
                    | jsonwebtoken::Algorithm::RS384
                    | jsonwebtoken::Algorithm::RS512
                    | jsonwebtoken::Algorithm::ES256
                    | jsonwebtoken::Algorithm::ES384
                    | jsonwebtoken::Algorithm::EdDSA => header.alg,
                    _ => {
                        return Err(crate::error::ConnectError::Provider(
                            "OIDC token header specifies an insecure or symmetric algorithm"
                                .to_string(),
                        ));
                    }
                };
                let validation =
                    crate::provider::id_token::validation(alg, &self.client_id, &[&self.issuer]);

                let token_data =
                    jsonwebtoken::decode::<Value>(id_token, &decoding_key, &validation).map_err(
                        |e| {
                            crate::error::ConnectError::Provider(format!(
                                "OIDC id_token signature or claims validation failed: {}",
                                e
                            ))
                        },
                    )?;
                let payload = token_data.claims;

                crate::provider::id_token::validate_claims(
                    &payload,
                    &self.client_id,
                    expected_nonce,
                )?;

                ConnectUser {
                    id: payload["sub"].as_str().map(String::from).ok_or_else(|| {
                        crate::error::ConnectError::Provider("Missing sub in id_token".to_owned())
                    })?,
                    name: payload["name"].as_str().map(String::from).ok_or_else(|| {
                        crate::error::ConnectError::Provider("Missing name in id_token".to_owned())
                    })?,
                    email: payload["email"].as_str().map(String::from),
                    avatar_url: payload["picture"].as_str().map(String::from),
                    email_verified: payload["email_verified"].as_bool(),
                    raw_data: payload,
                    access_token: secrecy::SecretString::from(access_token.to_owned()),
                    refresh_token: None,
                    expires_in: None,
                }
            } else {
                return Err(crate::error::ConnectError::Provider(
                    "Missing 'kid' header in OIDC id_token".to_owned(),
                ));
            }
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
        user.expires_in = crate::provider::token_lifetime(&token_res)?;

        Ok(user)
    }
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
            client_secret: Some(secrecy::ExposeSecret::expose_secret(&self.client_secret)),
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
        let form_data = [
            ("client_id", self.client_id.as_str()),
            (
                "client_secret",
                secrecy::ExposeSecret::expose_secret(&self.client_secret),
            ),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];
        self.get_user_from_form(&form_data, None).await
    }
}
