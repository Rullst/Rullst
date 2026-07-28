//! A local Mock Identity Provider for End-to-End Testing.
//!
//! Exposes an Axum router that perfectly simulates an OAuth2 / OIDC provider.
//! This allows developers to test their integration locally without internet access
//! or dealing with strict real-world rate limits and domain validation.

#[cfg(feature = "axum")]
use axum::{
    Json, Router,
    extract::{Form, Query},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
#[cfg(feature = "axum")]
use base64::Engine;
#[cfg(feature = "axum")]
use serde::Deserialize;
#[cfg(feature = "axum")]
use serde_json::json;

#[cfg(feature = "axum")]
#[derive(Deserialize)]
pub struct AuthQuery {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: Option<String>,
    pub state: Option<String>,
}

#[cfg(feature = "axum")]
#[derive(Deserialize)]
pub struct TokenForm {
    pub client_id: String,
    pub client_secret: String,
    pub code: String,
    pub grant_type: String,
    pub redirect_uri: String,
}

#[cfg(feature = "axum")]
/// Returns an `axum::Router` configured as a fake OAuth provider.
///
/// Endpoints:
/// - `GET /auth`: Redirects back with a static code.
/// - `POST /token`: Exchanges the code for fake access/id tokens.
/// - `GET /userinfo`: Returns a static fake user profile.
/// - `GET /.well-known/openid-configuration`: Returns the fake discovery document.
pub fn mock_router() -> Router {
    Router::new()
        .route("/auth", get(authorize_handler))
        .route("/token", post(token_handler))
        .route("/userinfo", get(userinfo_handler))
        .route("/.well-known/openid-configuration", get(discovery_handler))
}

#[cfg(feature = "axum")]
async fn authorize_handler(Query(params): Query<AuthQuery>) -> impl IntoResponse {
    let mut redirect = format!("{}?code=mock_auth_code_12345", params.redirect_uri);
    if let Some(state) = params.state {
        redirect = format!("{}&state={}", redirect, state);
    }
    Redirect::temporary(&redirect)
}

#[cfg(feature = "axum")]
async fn token_handler(Form(form): Form<TokenForm>) -> impl IntoResponse {
    if form.code != "mock_auth_code_12345" {
        return Json(json!({
            "error": "invalid_grant",
            "error_description": "The provided authorization code is invalid."
        }));
    }

    // A fake JWT for the id_token
    // ⚠️ WARNING: alg: none is ONLY safe for local testing. Never use in production.
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}");
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        b"{\"sub\":\"mock_user_999\",\"name\":\"Mock User\",\"email\":\"mock@example.com\",\"email_verified\":true}"
    );
    let id_token = format!("{}.{}.", header, payload);

    Json(json!({
        "access_token": "mock_access_token_abcde",
        "token_type": "Bearer",
        "expires_in": 3600,
        "refresh_token": "mock_refresh_token_fghij",
        "id_token": id_token
    }))
}

#[cfg(feature = "axum")]
async fn userinfo_handler() -> impl IntoResponse {
    Json(json!({
        "sub": "mock_user_999",
        "name": "Mock User",
        "email": "mock@example.com",
        "email_verified": true,
        "picture": "https://mock.provider/avatar.png"
    }))
}

#[cfg(feature = "axum")]
async fn discovery_handler() -> impl IntoResponse {
    Json(json!({
        "issuer": "http://localhost:8080",
        "authorization_endpoint": "http://localhost:8080/auth",
        "token_endpoint": "http://localhost:8080/token",
        "userinfo_endpoint": "http://localhost:8080/userinfo",
        "jwks_uri": "http://localhost:8080/jwks",
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        // ⚠️ WARNING: alg: none is ONLY safe for local testing. Never use in production.
        "id_token_signing_alg_values_supported": ["RS256", "none"]
    }))
}

#[cfg(all(test, feature = "axum"))]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn test_token_handler_invalid_code() {
        let form = TokenForm {
            client_id: "test".to_string(),
            client_secret: "test".to_string(),
            code: "invalid_code_here".to_string(),
            grant_type: "authorization_code".to_string(),
            redirect_uri: "http://test".to_string(),
        };

        let response = token_handler(axum::extract::Form(form))
            .await
            .into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body bytes");
        let json: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("Failed to parse response body as JSON");

        assert_eq!(json["error"], "invalid_grant");
    }

    #[tokio::test]
    async fn test_token_handler_valid_code() {
        let form = TokenForm {
            client_id: "test".to_string(),
            client_secret: "test".to_string(),
            code: "mock_auth_code_12345".to_string(),
            grant_type: "authorization_code".to_string(),
            redirect_uri: "http://test".to_string(),
        };

        let response = token_handler(axum::extract::Form(form))
            .await
            .into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body bytes");
        let json: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("Failed to parse response body as JSON");

        assert_eq!(json["access_token"], "mock_access_token_abcde");
    }

    #[tokio::test]
    async fn test_authorize_handler() {
        let query = AuthQuery {
            client_id: "test_client".to_string(),
            redirect_uri: "http://example.com/callback".to_string(),
            response_type: "code".to_string(),
            scope: Some("openid profile".to_string()),
            state: Some("test_state".to_string()),
        };

        let response = authorize_handler(axum::extract::Query(query))
            .await
            .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::TEMPORARY_REDIRECT
        );
        let headers = response.headers();
        let location = headers
            .get(axum::http::header::LOCATION)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(
            location,
            "http://example.com/callback?code=mock_auth_code_12345&state=test_state"
        );
    }

    #[tokio::test]
    async fn test_mock_router_discovery() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let app = mock_router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/.well-known/openid-configuration")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["issuer"], "http://localhost:8080");
        assert_eq!(json["authorization_endpoint"], "http://localhost:8080/auth");
    }

    #[tokio::test]
    async fn test_mock_router_userinfo() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let app = mock_router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/userinfo")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["sub"], "mock_user_999");
        assert_eq!(json["email"], "mock@example.com");
    }
}
