use async_trait::async_trait;
use serde_json::Value;

use crate::client::HttpClientExt;
use crate::error::ConnectError;
use crate::provider::Provider;
use crate::user::ConnectUser;

use super::discovery::OidcProvider;

impl OidcProvider {
    pub(crate) async fn get_jwks(
        &self,
    ) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, ConnectError> {
        crate::provider::fetch_and_cache_jwks(&self.jwks_uri, self.http_client.as_ref()).await
    }

    #[tracing::instrument(skip(self, form_data))]
    pub(crate) async fn get_user_from_form(
        &self,
        form_data: &crate::provider::TokenExchangeForm<'_>,
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
                let jwks = self.get_jwks().await?;
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
                let mut validation = jsonwebtoken::Validation::new(alg);
                validation.set_audience(&[&self.client_id]);
                validation.set_issuer(&[&self.issuer]);
                validation.validate_exp = true;
                if expected_nonce.is_some() {
                    validation.set_required_spec_claims(&["nonce"]);
                }

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

                if let Some(nonce) = expected_nonce {
                    let token_nonce = payload["nonce"].as_str().unwrap_or("");
                    if !crate::provider::verify_nonce(token_nonce, nonce) {
                        return Err(crate::error::ConnectError::Provider(
                            "OIDC id_token nonce mismatch".to_owned(),
                        ));
                    }
                }

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
            use crate::provider::Provider;
            self.get_user_from_token(access_token).await?
        };

        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = token_res["expires_in"]
            .as_u64()
            .or_else(|| token_res["expires_in"].as_i64().map(|v| v as u64));

        Ok(user)
    }
}

#[async_trait]
impl Provider for OidcProvider {
    crate::impl_standard_redirect_url!("{}");

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
        let form_data = crate::provider::TokenExchangeForm {
            client_id: self.client_id.as_str(),
            client_secret: Some(secrecy::ExposeSecret::expose_secret(&self.client_secret)),
            code: refresh_token,
            grant_type: Some("refresh_token"),
            redirect_uri: self.redirect_url.as_str(),
            code_verifier: None,
        };
        self.get_user_from_form(&form_data, None).await
    }
}
