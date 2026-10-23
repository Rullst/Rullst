//! Microsoft identity platform sign-in with OpenID Connect: Entra ID work and
//! school accounts and personal Microsoft accounts, for an explicitly chosen
//! [`MicrosoftTenant`]. Added in v13.
//!
//! Unlike [`super::MicrosoftProvider`], which reads Microsoft Graph `/me`
//! through the `common` authority, this provider verifies the ID token of the
//! authorization-code exchange: its RS256 signature through the tenant's
//! rotating keys, the audience (this `client_id`), lifetime and nonce, and an
//! issuer of the form `https://login.microsoftonline.com/<tid>/v2.0` whose
//! directory (tenant) ID `tid` the configured tenant admits.
//!
//! Identity facts:
//! - [`ConnectUser::id`] is `<tid>:<oid>`, the directory ID and the user's
//!   object ID: together they are Microsoft's stable key for a user, the same
//!   for every application registration. (`sub` is pairwise: it changes with
//!   the application ID.) The `profile` scope, requested by default, releases
//!   `oid`.
//! - [`ConnectUser::email`] is the optional `email` claim. Microsoft does not
//!   verify it: an organization's administrators can give a user any address.
//!   `email_verified` is `Some(true)` only when the token carries the optional
//!   `xms_edov` claim with the value `true` (Microsoft verified that the
//!   address's domain belongs to the user's organization, or the account is a
//!   personal or Google account); the application registration must add
//!   `xms_edov` and `email` as optional ID-token claims. Otherwise it is
//!   `None`, and the email must not link or create accounts.
//! - `name` is the `name` claim, which may be empty.

use crate::client::{HttpClient, HttpClientExt};
use crate::configuration::CredentialMode;
use crate::error::ConnectError;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use secrecy::SecretString;
use serde_json::Value;
use std::sync::Arc;

/// The directory ID Microsoft uses for every personal Microsoft account.
pub const MICROSOFT_CONSUMER_TENANT_ID: &str = "9188040d-6c67-4c5b-b112-36a304b66dad";

const AUTHORITY: &str = "https://login.microsoftonline.com";

/// Which accounts may sign in with [`MicrosoftOidcProvider`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MicrosoftTenant {
    /// Accounts of any Entra ID organization and personal Microsoft accounts.
    Common,
    /// Accounts of any Entra ID organization; personal accounts are refused.
    Organizations,
    /// Personal Microsoft accounts only.
    Consumers,
    /// Accounts of one Entra ID organization, by its directory (tenant) ID.
    Tenant(String),
}

impl MicrosoftTenant {
    /// Parses `common`, `organizations`, `consumers` or a directory (tenant)
    /// ID in its GUID form, such as `72f988bf-86f1-41af-91ab-2d7cd011db47`.
    /// Domain names are refused: an ID token names its tenant only by GUID.
    pub fn parse(value: &str) -> Result<Self, ConnectError> {
        let value = value.trim().to_ascii_lowercase();
        match value.as_str() {
            "common" => Ok(Self::Common),
            "organizations" => Ok(Self::Organizations),
            "consumers" => Ok(Self::Consumers),
            id if is_guid(id) => Ok(Self::Tenant(value)),
            _ => Err(crate::configuration::invalid(
                "tenant",
                "use common, organizations, consumers or a directory (tenant) ID GUID",
            )),
        }
    }

    /// The authority path segment: `common`, `organizations`, `consumers` or
    /// the directory ID.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Common => "common",
            Self::Organizations => "organizations",
            Self::Consumers => "consumers",
            Self::Tenant(id) => id,
        }
    }

    /// Whether an account of the directory `tid` may sign in.
    fn admits(&self, tid: &str) -> bool {
        match self {
            Self::Common => true,
            Self::Organizations => tid != MICROSOFT_CONSUMER_TENANT_ID,
            Self::Consumers => tid == MICROSOFT_CONSUMER_TENANT_ID,
            Self::Tenant(id) => tid == id,
        }
    }
}

/// A lowercase `8-4-4-4-12` hexadecimal GUID.
fn is_guid(value: &str) -> bool {
    let groups: Vec<&str> = value.split('-').collect();
    groups.len() == 5
        && groups.iter().zip([8, 4, 4, 4, 12]).all(|(group, length)| {
            group.len() == length
                && group
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

/// Microsoft identity platform OpenID Connect provider; see the module
/// documentation for the identity it returns.
pub struct MicrosoftOidcProvider {
    tenant: MicrosoftTenant,
    client_id: String,
    client_secret: SecretString,
    redirect_url: String,
    http_client: Arc<dyn HttpClient>,
    credential_mode: CredentialMode,
    jwks_cache: crate::provider::JwksCache,
}

impl MicrosoftOidcProvider {
    /// A validated provider for `tenant`. Empty or `mock_*` credentials select
    /// the offline mock (with the `mock` feature; otherwise they are rejected
    /// for the field `credentials`).
    pub fn try_new(
        tenant: MicrosoftTenant,
        client_id: impl Into<String>,
        client_secret: SecretString,
        redirect_url: impl Into<String>,
    ) -> Result<Self, ConnectError> {
        let client_id = client_id.into();
        let redirect_url = redirect_url.into();
        crate::configuration::validate_redirect_url(&redirect_url)?;
        let credential_mode =
            crate::configuration::checked_credential_mode(&client_id, &client_secret)?;
        let http_client =
            crate::configuration::provider_http_client(credential_mode, "microsoft", None);
        Ok(Self {
            tenant,
            client_id,
            client_secret,
            redirect_url,
            http_client,
            credential_mode,
            jwks_cache: crate::provider::JwksCache::default(),
        })
    }

    /// The accounts this provider admits.
    #[must_use]
    pub fn tenant(&self) -> &MicrosoftTenant {
        &self.tenant
    }

    /// Whether this instance uses live or mock credentials.
    #[must_use]
    pub fn credential_mode(&self) -> CredentialMode {
        self.credential_mode
    }

    /// Sets the HTTP transport for live credentials (a proxy or a test
    /// transport); mock credentials keep their network-free transport.
    #[must_use]
    pub fn with_http_client(mut self, client: Arc<dyn HttpClient>) -> Self {
        if matches!(self.credential_mode, CredentialMode::Live) {
            self.http_client = client;
        }
        self
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{AUTHORITY}/{}/{path}", self.tenant.as_str())
    }

    /// Verifies the ID token and returns its claims; see the module docs.
    async fn verify_id_token(
        &self,
        id_token: &str,
        expected_nonce: Option<&str>,
    ) -> Result<Value, ConnectError> {
        let header = jsonwebtoken::decode_header(id_token)?;
        if header.alg != jsonwebtoken::Algorithm::RS256 {
            return Err(rejected("unsupported ID token algorithm"));
        }
        let kid = header
            .kid
            .ok_or_else(|| rejected("the ID token header has no key ID"))?;
        let jwks = self
            .jwks_cache
            .get_for_kid(
                &self.endpoint("discovery/v2.0/keys"),
                &kid,
                self.http_client.as_ref(),
            )
            .await?;
        let jwk = jwks
            .find(&kid)
            .ok_or_else(|| ConnectError::JwkNotFound(kid.clone()))?;
        let key = jsonwebtoken::DecodingKey::from_jwk(jwk)?;
        let mut validation = crate::provider::id_token::validation(
            jsonwebtoken::Algorithm::RS256,
            &self.client_id,
            &[],
        );
        // The issuer names the user's directory; it is checked below.
        validation.iss = None;
        let claims = jsonwebtoken::decode::<Value>(id_token, &key, &validation)
            .map_err(|error| rejected(&format!("ID token validation failed: {error}")))?
            .claims;
        let tid = claims["tid"]
            .as_str()
            .filter(|tid| is_guid(tid))
            .ok_or_else(|| rejected("the ID token has no directory (tenant) ID"))?;
        if claims["iss"].as_str() != Some(&format!("{AUTHORITY}/{tid}/v2.0")) {
            return Err(rejected("the ID token issuer does not match its tenant"));
        }
        if !self.tenant.admits(tid) {
            return Err(rejected("this account's directory may not sign in here"));
        }
        crate::provider::id_token::validate_claims(&claims, &self.client_id, expected_nonce)?;
        Ok(claims)
    }

    fn mock_user(&self) -> Result<ConnectUser, ConnectError> {
        if !crate::configuration::MOCK_IDENTITIES {
            return Err(ConnectError::Offline(
                "mock credentials need the 'mock' feature".to_string(),
            ));
        }
        let tid = match &self.tenant {
            MicrosoftTenant::Tenant(id) => id.as_str(),
            MicrosoftTenant::Consumers => MICROSOFT_CONSUMER_TENANT_ID,
            _ => "00000000-0000-0000-0000-00000000000a",
        };
        user_from_claims(
            serde_json::json!({
                "tid": tid,
                "oid": "00000000-0000-0000-0000-000000000001",
                "name": "Rullst Mock User",
                "email": "mock@example.invalid",
                "xms_edov": true,
            }),
            SecretString::from("mock_access_token".to_owned()),
        )
    }
}

fn rejected(reason: &str) -> ConnectError {
    ConnectError::Provider(format!("Microsoft sign-in refused: {reason}"))
}

/// The user of verified claims: `<tid>:<oid>`, `name`, `email`, and
/// `email_verified` from `xms_edov`.
fn user_from_claims(
    claims: Value,
    access_token: SecretString,
) -> Result<ConnectUser, ConnectError> {
    let oid = claims["oid"]
        .as_str()
        .filter(|oid| !oid.is_empty())
        .ok_or_else(|| rejected("the ID token has no object ID (request the profile scope)"))?;
    let tid = claims["tid"].as_str().unwrap_or_default();
    Ok(ConnectUser {
        id: format!("{tid}:{oid}"),
        name: claims["name"].as_str().unwrap_or_default().to_owned(),
        email: claims["email"].as_str().map(str::to_owned),
        avatar_url: None,
        email_verified: (crate::user::email_verified_claim(&claims["xms_edov"]) == Some(true))
            .then_some(true),
        raw_data: claims,
        access_token,
        refresh_token: None,
        expires_in: None,
    })
}

#[async_trait]
impl Provider for MicrosoftOidcProvider {
    fn redirect_url(&self) -> String {
        if self.credential_mode.is_mock() {
            return crate::configuration::mock_redirect_url(&self.redirect_url, None, None);
        }
        let authorize = self.endpoint("oauth2/v2.0/authorize");
        let mut params = crate::provider::build_oauth_params(
            &authorize,
            &self.client_id,
            &self.redirect_url,
            "openid profile email",
            None,
            None,
        );
        params.append_pair("response_type", "code");
        params.append_pair("response_mode", "query");
        params.finish()
    }

    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, ConnectError> {
        if self.credential_mode.is_mock() {
            return self.mock_user();
        }
        let form = crate::provider::TokenExchangeForm {
            client_id: &self.client_id,
            client_secret: Some(secrecy::ExposeSecret::expose_secret(&self.client_secret)),
            code: params.auth_code,
            grant_type: Some("authorization_code"),
            redirect_uri: &self.redirect_url,
            code_verifier: params.code_verifier,
        };
        let token = self
            .http_client
            .post(self.token_url())
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        let id_token = token["id_token"]
            .as_str()
            .ok_or_else(|| rejected("the token response has no ID token"))?;
        let claims = self
            .verify_id_token(id_token, params.expected_nonce)
            .await?;
        let access_token = token["access_token"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let mut user = user_from_claims(claims, SecretString::from(access_token))?;
        user.refresh_token = token["refresh_token"]
            .as_str()
            .map(|token| SecretString::from(token.to_owned()));
        user.expires_in = crate::provider::token_lifetime(&token)?;
        Ok(user)
    }

    /// Not supported: a Microsoft identity comes only from a verified ID
    /// token of this application's code exchange ([`Provider::get_user`]).
    async fn get_user_from_token(&self, _access_token: &str) -> Result<ConnectUser, ConnectError> {
        Err(rejected(
            "an access token does not identify a user; sign in with the authorization code",
        ))
    }

    fn token_url(&self) -> String {
        self.endpoint("oauth2/v2.0/token")
    }
}

#[cfg(test)]
#[path = "microsoft_oidc_tests.rs"]
mod tests;
