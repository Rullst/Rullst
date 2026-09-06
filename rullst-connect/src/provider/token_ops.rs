//! Token exchange and user profile resolution helpers.

use crate::client::HttpClientExt;
use crate::provider::traits::Provider;
use crate::provider::types::{Oauth2TokenResponse, TokenExchangeForm};
use crate::user::ConnectUser;

const MAX_REVOCATION_TOKEN_BYTES: usize = 16 * 1024;

pub(crate) fn validate_revocation_token(token: &str) -> Result<(), crate::error::ConnectError> {
    if token.is_empty()
        || token.len() > MAX_REVOCATION_TOKEN_BYTES
        || token.trim().len() != token.len()
        || token.chars().any(char::is_control)
    {
        return Err(crate::error::ConnectError::Token(
            "revocation token must be non-empty, bounded, and free of whitespace padding or control characters"
                .to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn require_oauth_only(
    expected_nonce: Option<&str>,
) -> Result<(), crate::error::ConnectError> {
    if expected_nonce.is_some() {
        return Err(crate::error::ConnectError::Provider(
            "this OAuth-only adapter cannot validate an OIDC nonce; use Google, Apple or OidcProvider for ID-token login".into(),
        ));
    }
    Ok(())
}

pub(crate) fn unsupported_revocation_kind(
    provider: &'static str,
    kind: crate::provider::RevocationTokenKind,
) -> crate::error::ConnectError {
    crate::error::ConnectError::Token(format!(
        "{provider} does not support {} revocation through this adapter",
        kind.as_str()
    ))
}

pub(crate) fn provider_url_with_segments(
    base: &str,
    segments: &[&str],
) -> Result<String, crate::error::ConnectError> {
    let mut url = url::Url::parse(base).map_err(|_| {
        crate::error::ConnectError::Provider("provider endpoint is invalid".to_string())
    })?;
    {
        let mut path = url.path_segments_mut().map_err(|_| {
            crate::error::ConnectError::Provider(
                "provider endpoint cannot contain path segments".to_string(),
            )
        })?;
        path.extend(segments.iter().copied());
    }
    Ok(url.to_string())
}

pub(crate) async fn revoke_form_with_body_credentials(
    client: &dyn crate::client::HttpClient,
    endpoint: impl Into<String>,
    client_id: &str,
    client_secret: &str,
    token: &str,
    kind: Option<crate::provider::RevocationTokenKind>,
) -> Result<(), crate::error::ConnectError> {
    validate_revocation_token(token)?;
    let mut form = vec![
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("token", token),
    ];
    if let Some(kind) = kind {
        form.push(("token_type_hint", kind.as_str()));
    }
    client
        .post(endpoint)
        .form(&form)
        .send()
        .await?
        .error_for_status_redacted("token revocation")?;
    Ok(())
}

pub(crate) async fn revoke_form_with_basic_credentials(
    client: &dyn crate::client::HttpClient,
    endpoint: impl Into<String>,
    client_id: &str,
    client_secret: &str,
    token: &str,
    kind: Option<crate::provider::RevocationTokenKind>,
) -> Result<(), crate::error::ConnectError> {
    validate_revocation_token(token)?;
    let mut form = vec![("token", token)];
    if let Some(kind) = kind {
        form.push(("token_type_hint", kind.as_str()));
    }
    client
        .post(endpoint)
        .basic_auth(client_id, Some(client_secret))
        .form(&form)
        .send()
        .await?
        .error_for_status_redacted("token revocation")?;
    Ok(())
}

pub(crate) async fn revoke_json_with_basic_delete(
    client: &dyn crate::client::HttpClient,
    endpoint: impl Into<String>,
    client_id: &str,
    client_secret: &str,
    token: &str,
) -> Result<(), crate::error::ConnectError> {
    validate_revocation_token(token)?;
    client
        .delete(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "rullst-connect")
        .basic_auth(client_id, Some(client_secret))
        .json(serde_json::json!({ "access_token": token }))
        .send()
        .await?
        .error_for_status_redacted("token revocation")?;
    Ok(())
}

/// Helper to exchange an authorization code for access tokens using standard OAuth2.
pub async fn fetch_access_token(
    client: &dyn crate::client::HttpClient,
    token_url: &str,
    form: &TokenExchangeForm<'_>,
) -> Result<Oauth2TokenResponse, crate::error::ConnectError> {
    let token_res = client
        .post(token_url)
        .form(form)
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    if let Some(err) = token_res["error"].as_str() {
        let err_desc = token_res["error_description"].as_str().unwrap_or("");
        return Err(crate::error::ConnectError::Token(format!(
            "Provider returned error: {} - {}",
            err, err_desc
        )));
    }

    let access_token = token_res["access_token"]
        .as_str()
        .ok_or_else(|| crate::error::ConnectError::Token("Failed to get access_token".to_owned()))?
        .to_owned();

    let refresh_token = token_res["refresh_token"].as_str().map(String::from);
    let expires_in = crate::provider::token_lifetime(&token_res)?;

    Ok(Oauth2TokenResponse {
        access_token,
        refresh_token,
        expires_in,
    })
}

/// Helper to exchange a refresh token for new access tokens using standard OAuth2.
pub async fn fetch_refresh_token(
    client: &dyn crate::client::HttpClient,
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<Oauth2TokenResponse, crate::error::ConnectError> {
    let token_res = client
        .post(token_url)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    if let Some(err) = token_res["error"].as_str() {
        let err_desc = token_res["error_description"].as_str().unwrap_or("");
        return Err(crate::error::ConnectError::Token(format!(
            "Provider returned error: {} - {}",
            err, err_desc
        )));
    }

    let access_token = token_res["access_token"]
        .as_str()
        .ok_or_else(|| {
            crate::error::ConnectError::Token(
                "Failed to get access_token during refresh".to_owned(),
            )
        })?
        .to_owned();

    let refresh_token = token_res["refresh_token"].as_str().map(String::from);
    let expires_in = crate::provider::token_lifetime(&token_res)?;

    Ok(Oauth2TokenResponse {
        access_token,
        refresh_token,
        expires_in,
    })
}

/// Helper to exchange an authorization code and build the ConnectUser profile.
#[allow(clippy::too_many_arguments)]
pub async fn exchange_and_get_user<P>(
    provider: &P,
    client: &dyn crate::client::HttpClient,
    token_url: &str,
    form: &TokenExchangeForm<'_>,
    expected_nonce: Option<&str>,
) -> Result<ConnectUser, crate::error::ConnectError>
where
    P: Provider + ?Sized,
{
    require_oauth_only(expected_nonce)?;
    let token = fetch_access_token(client, token_url, form).await?;

    let mut user = provider.get_user_from_token(&token.access_token).await?;
    user.refresh_token = token.refresh_token.map(secrecy::SecretString::from);
    user.expires_in = token.expires_in;
    Ok(user)
}

/// Helper to refresh an access token and fetch the updated ConnectUser profile.
pub async fn refresh_and_get_user<P>(
    provider: &P,
    client: &dyn crate::client::HttpClient,
    token_url: &str,
    client_id: &str,
    client_secret: &secrecy::SecretString,
    refresh_token: &str,
) -> Result<ConnectUser, crate::error::ConnectError>
where
    P: Provider + ?Sized,
{
    let token = fetch_refresh_token(
        client,
        token_url,
        client_id,
        secrecy::ExposeSecret::expose_secret(client_secret),
        refresh_token,
    )
    .await?;

    let mut user = provider.get_user_from_token(&token.access_token).await?;
    user.refresh_token = token.refresh_token.map(secrecy::SecretString::from);
    user.expires_in = token.expires_in;
    Ok(user)
}
pub(crate) fn validate_token_lifetime(
    lifetime: Option<u64>,
) -> Result<Option<u64>, crate::ConnectError> {
    if lifetime
        .is_some_and(|seconds| seconds == 0 || seconds > crate::refresh::MAX_TOKEN_LIFETIME_SECONDS)
    {
        return Err(crate::ConnectError::Token(
            "expires_in must be between 1 second and 366 days".into(),
        ));
    }
    Ok(lifetime)
}

pub(crate) fn token_lifetime(
    response: &serde_json::Value,
) -> Result<Option<u64>, crate::ConnectError> {
    let Some(value) = response.get("expires_in").filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let seconds = value.as_u64().ok_or_else(|| {
        crate::ConnectError::Token("expires_in must be a positive integer".into())
    })?;
    validate_token_lifetime(Some(seconds))
}
