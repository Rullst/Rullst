use serde::Deserialize;

#[cfg(feature = "axum-session")]
mod session;
#[cfg(feature = "axum-session")]
pub use session::{
    AuthSession, AuthSessionForm, OAuthAuthorization, begin_oauth_session, begin_oidc_session,
};

/// Standard OAuth2 callback query parameters.
///
/// Axum and Actix integrations provide native extractors behind their features.
/// Other hosts can deserialize their bounded callback query into this plain
/// Serde type and must still validate the managed state/session lifecycle.
///
/// # Example (Axum)
/// ```rust,no_run
/// use axum::{
///     extract::Query,
///     response::{IntoResponse, Response},
/// };
/// use rullst_connect::extractors::AuthCallback;
///
/// async fn auth_callback(Query(params): Query<AuthCallback>) -> Response {
///     if let Some(error) = params.error {
///         return format!("Auth failed: {error}").into_response();
///     }
///
///     let Some(code) = params.code else {
///         return "Authorization code missing".into_response();
///     };
///     format!("Exchange authorization code: {code}").into_response()
/// }
/// ```
#[derive(Deserialize, Clone, PartialEq)]
pub struct AuthCallback {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// Redacts the authorization code and CSRF state, which are single-use secrets.
impl std::fmt::Debug for AuthCallback {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const REDACTED: &str = "[REDACTED]";
        formatter
            .debug_struct("AuthCallback")
            .field("code", &self.code.as_ref().map(|_| REDACTED))
            .field("state", &self.state.as_ref().map(|_| REDACTED))
            .field("error", &self.error)
            .field("error_description", &self.error_description)
            .finish()
    }
}

impl AuthCallback {
    /// Helper to verify the CSRF state parameter.
    pub fn verify_state(&self, session_state: &str) -> Result<(), crate::error::ConnectError> {
        use sha2::{Digest, Sha256};
        use subtle::ConstantTimeEq;

        match &self.state {
            Some(state) if !state.is_empty() && !session_state.is_empty() => {
                let hash_state = Sha256::digest(state.as_bytes());
                let hash_session = Sha256::digest(session_state.as_bytes());

                if bool::from(hash_state.ct_eq(&hash_session)) {
                    Ok(())
                } else {
                    Err(crate::error::ConnectError::InvalidState(
                        "CSRF state mismatch".into(),
                    ))
                }
            }
            Some(_) => Err(crate::error::ConnectError::InvalidState(
                "CSRF state cannot be empty".into(),
            )),
            None => Err(crate::error::ConnectError::InvalidState(
                "State missing in callback".into(),
            )),
        }
    }
}

#[cfg(feature = "axum")]
impl<S> axum::extract::FromRequestParts<S> for AuthCallback
where
    S: Send + Sync,
{
    type Rejection = axum::extract::rejection::QueryRejection;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(callback) =
            axum::extract::Query::<AuthCallback>::from_request_parts(parts, state).await?;
        Ok(callback)
    }
}

#[cfg(feature = "actix")]
impl actix_web::FromRequest for AuthCallback {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        match actix_web::web::Query::<AuthCallback>::from_query(req.query_string()) {
            Ok(query) => std::future::ready(Ok(query.into_inner())),
            Err(e) => std::future::ready(Err(e.into())),
        }
    }
}

#[cfg(test)]
mod tests;
