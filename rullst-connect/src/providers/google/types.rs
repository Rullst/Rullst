//! Google OAuth2 and OIDC token response data transfer objects.

#[derive(serde::Deserialize)]
pub(crate) struct GoogleTokenResponse {
    pub access_token: String,
    pub id_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}
