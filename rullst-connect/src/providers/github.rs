//! GitHub OAuth 2.0 adapter (GitHub is not an OpenID Connect provider).
//!
//! The identity comes from the REST API: [`ConnectUser::id`] is the numeric
//! account `id` of `GET /user`, which never changes, never the `login`, which
//! the user can rename. The email is the address `GET /user/emails` marks
//! `primary`, and `email_verified` is that entry's `verified` flag; the
//! default `user:email` scope grants the call. When the call fails or lists
//! no primary address, `email` is the public profile address of `/user` and
//! `email_verified` is `None`, so it never links or creates an account.

use crate::client::HttpClientExt;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use serde_json::Value;

crate::define_provider!(GithubProvider, "user:email");

/// The account's primary address in a `GET /user/emails` response, with
/// GitHub's `verified` flag; `None` when no entry is marked primary.
fn primary_email(emails: &Value) -> Option<(String, bool)> {
    emails.as_array()?.iter().find_map(|entry| {
        if entry["primary"].as_bool() != Some(true) {
            return None;
        }
        let address = entry["email"].as_str()?;
        Some((
            address.to_owned(),
            entry["verified"].as_bool() == Some(true),
        ))
    })
}

impl GithubProvider {
    /// The primary address and its `verified` flag from `GET /user/emails`;
    /// `None` when the request fails, for example without `user:email`.
    async fn primary_email(&self, access_token: &str) -> Option<(String, bool)> {
        let emails = self
            .http_client
            .get("https://api.github.com/user/emails")
            .bearer_auth(access_token)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "rullst-connect")
            .send()
            .await
            .and_then(|response| response.error_for_status());
        match emails {
            Ok(response) => primary_email(&response.json::<Value>().await.ok()?),
            Err(error) => {
                tracing::warn!("GitHub /user/emails failed; the email stays unverified: {error}");
                None
            }
        }
    }

    async fn get_user_from_form(
        &self,
        form_data: &crate::provider::TokenExchangeForm<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let token_res = self
            .http_client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(form_data)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        if let Some(err) = token_res["error"].as_str() {
            let err_desc = token_res["error_description"].as_str().unwrap_or_default();
            return Err(crate::error::provider_returned_error(err, err_desc));
        }

        let access_token = token_res["access_token"].as_str().ok_or_else(|| {
            crate::error::ConnectError::Token("Failed to get access_token".to_string())
        })?;

        let mut user = self.get_user_from_token(access_token).await?;
        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = crate::provider::token_lifetime(&token_res)?;
        Ok(user)
    }
}

#[async_trait]
impl Provider for GithubProvider {
    crate::impl_standard_redirect_url!(
        "https://github.com/login/oauth/authorize?response_type=code"
    );

    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        crate::provider::require_oauth_only(params.expected_nonce)?;
        let form_data = crate::provider::TokenExchangeForm {
            client_id: self.client_id.as_str(),
            client_secret: Some(secrecy::ExposeSecret::expose_secret(&self.client_secret)),
            code: params.auth_code,
            grant_type: None,
            redirect_uri: self.redirect_url.as_str(),
            code_verifier: params.code_verifier,
        };
        self.get_user_from_form(&form_data).await
    }

    async fn get_user_from_token(
        &self,
        access_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let user_res = self
            .http_client
            .get("https://api.github.com/user")
            .bearer_auth(access_token)
            .header("User-Agent", "rullst-connect")
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let id = user_res["id"]
            .as_i64()
            .ok_or_else(|| crate::error::ConnectError::Provider("Missing id".to_string()))?
            .to_string();
        let (email, email_verified) = match self.primary_email(access_token).await {
            Some((address, verified)) => (Some(address), Some(verified)),
            None => (user_res["email"].as_str().map(String::from), None),
        };
        Ok(ConnectUser {
            id,
            name: user_res["name"]
                .as_str()
                .unwrap_or(user_res["login"].as_str().unwrap_or(""))
                .to_string(),
            email,
            avatar_url: user_res["avatar_url"].as_str().map(String::from),
            email_verified,
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_string()),
            refresh_token: None,
            expires_in: None,
        })
    }

    fn token_url(&self) -> String {
        "https://github.com/login/oauth/access_token".to_string()
    }

    crate::impl_standard_refresh_token!();

    async fn revoke_token_with_kind(
        &self,
        token: &str,
        kind: crate::provider::RevocationTokenKind,
    ) -> Result<(), crate::error::ConnectError> {
        if kind != crate::provider::RevocationTokenKind::AccessToken {
            return Err(crate::provider::unsupported_revocation_kind("GitHub", kind));
        }
        let endpoint = crate::provider::provider_url_with_segments(
            "https://api.github.com/",
            &["applications", &self.client_id, "token"],
        )?;
        crate::provider::revoke_json_with_basic_delete(
            self.http_client.as_ref(),
            endpoint,
            &self.client_id,
            secrecy::ExposeSecret::expose_secret(&self.client_secret),
            token,
        )
        .await
    }

    async fn request_device_code(
        &self,
    ) -> Result<crate::user::DeviceAuthorizationResponse, crate::error::ConnectError> {
        let mut form = vec![("client_id", self.client_id.as_str())];
        if !self.scopes.is_empty() {
            form.push(("scope", self.scopes.as_str()));
        }

        let res = self
            .http_client
            .post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let device_code = res["device_code"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                crate::error::ConnectError::Provider(
                    "Missing device_code from Github response".to_string(),
                )
            })?;
        let user_code = res["user_code"].as_str().map(String::from).ok_or_else(|| {
            crate::error::ConnectError::Provider(
                "Missing user_code from Github response".to_string(),
            )
        })?;
        let verification_uri = res["verification_uri"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                crate::error::ConnectError::Provider(
                    "Missing verification_uri from Github response".to_string(),
                )
            })?;

        Ok(crate::user::DeviceAuthorizationResponse {
            device_code,
            user_code,
            verification_uri,
            verification_uri_complete: res["verification_uri_complete"].as_str().map(String::from),
            expires_in: res["expires_in"].as_u64().unwrap_or(900),
            interval: res["interval"].as_u64(),
        })
    }

    async fn poll_device_token(
        &self,
        device_code: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let token_res = self
            .http_client
            .post(self.token_url())
            .header("Accept", "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        if let Some(err) = token_res["error"].as_str() {
            let err_desc = token_res["error_description"].as_str().unwrap_or_default();
            return Err(crate::error::provider_returned_error(err, err_desc));
        }

        let access_token = token_res["access_token"].as_str().ok_or_else(|| {
            crate::error::ConnectError::Token(
                "Failed to get access_token during device poll. (Authorization pending?)"
                    .to_string(),
            )
        })?;

        let mut user = self.get_user_from_token(access_token).await?;
        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = crate::provider::token_lifetime(&token_res)?;
        Ok(user)
    }
}

#[cfg(test)]
#[path = "github_tests.rs"]
mod tests;
