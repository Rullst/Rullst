//! Asynchronous `Provider` trait implementation for Google.

use crate::client::HttpClientExt;
use crate::provider::Provider;
use crate::providers::google::provider::GoogleProvider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
impl Provider for GoogleProvider {
    crate::impl_standard_redirect_url!("https://accounts.google.com/o/oauth2/v2/auth");

    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
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

    async fn get_user_from_token(
        &self,
        access_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        // Fetch user profile
        let user_res = self
            .http_client
            .get("https://www.googleapis.com/oauth2/v3/userinfo")
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
            name: user_res["name"]
                .as_str()
                .map(String::from)
                .unwrap_or_default(),
            email: user_res["email"].as_str().map(String::from),
            avatar_url: user_res["picture"]
                .as_str()
                .map(|s: &str| s.replace("=s96-c", "=s400-c")),
            email_verified: user_res["email_verified"].as_bool(),
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_owned()),
            refresh_token: None,
            expires_in: None,
        })
    }

    async fn revoke_token_with_kind(
        &self,
        token: &str,
        _kind: crate::provider::RevocationTokenKind,
    ) -> Result<(), crate::error::ConnectError> {
        crate::provider::validate_revocation_token(token)?;
        self.http_client
            .post("https://oauth2.googleapis.com/revoke")
            .form(&[("token", token)])
            .send()
            .await?
            .error_for_status_redacted("token revocation")?;
        Ok(())
    }

    fn token_url(&self) -> String {
        "https://oauth2.googleapis.com/token".to_string()
    }

    crate::impl_standard_refresh_token!();
}
