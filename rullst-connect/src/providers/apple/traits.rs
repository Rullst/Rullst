//! Asynchronous `Provider` trait implementation for Apple.

use crate::client::HttpClientExt;
use crate::provider::Provider;
use crate::providers::apple::provider::AppleProvider;
use crate::user::ConnectUser;
use async_trait::async_trait;

#[async_trait]
impl Provider for AppleProvider {
    fn redirect_url(&self) -> String {
        let mut params = crate::provider::build_oauth_params(
            "https://appleid.apple.com/auth/authorize",
            &self.client_id,
            &self.redirect_url,
            &self.scopes,
            self.state.as_deref(),
            self.pkce_challenge.as_deref(),
        );
        params.append_pair("response_type", "code");
        params.append_pair("response_mode", "form_post");
        params.finish()
    }

    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let client_secret = self.generate_client_secret()?;
        let form_data = crate::provider::TokenExchangeForm {
            client_id: self.client_id.as_str(),
            client_secret: Some(client_secret.as_str()),
            code: params.auth_code,
            grant_type: Some("authorization_code"),
            redirect_uri: self.redirect_url.as_str(),
            code_verifier: params.code_verifier,
        };
        self.get_user_from_form(&form_data, params.expected_nonce)
            .await
    }

    /// For Apple, `access_token` parameter should actually be the `id_token` JWT string.
    async fn get_user_from_token(
        &self,
        id_token_str: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        self.decode_apple_id_token(id_token_str, None).await
    }

    fn token_url(&self) -> String {
        "https://appleid.apple.com/auth/token".to_string()
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let client_secret = self.generate_client_secret()?;

        let token_res = self
            .http_client
            .post(self.token_url())
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;

        if let Some(err) = token_res["error"].as_str() {
            let err_desc = token_res["error_description"].as_str().unwrap_or_default();
            return Err(crate::error::ConnectError::Token(format!(
                "Provider returned error: {} - {}",
                err, err_desc
            )));
        }

        let access_token = token_res["access_token"].as_str().ok_or_else(|| {
            crate::error::ConnectError::Token(
                "Failed to get access_token during refresh".to_string(),
            )
        })?;

        let mut user = self.get_user_from_token(access_token).await?;
        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = token_res["expires_in"]
            .as_u64()
            .or_else(|| token_res["expires_in"].as_i64().map(|v| v as u64));
        Ok(user)
    }
}
