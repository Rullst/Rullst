//! Deterministic clients used by explicit mock and fail-closed provider modes.

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{HttpClient, HttpRequest, HttpResponse};
use crate::error::ConnectError;

/// Network-free transport selected for empty or `mock_*` credentials.
pub(crate) struct OfflineHttpClient {
    provider: &'static str,
}

impl OfflineHttpClient {
    pub(crate) fn new(provider: &'static str) -> Self {
        Self { provider }
    }

    fn token_response() -> Value {
        json!({
            "access_token": "mock_access_token",
            "refresh_token": "mock_refresh_token",
            "expires_in": 3600
        })
    }

    fn user_response(&self) -> Value {
        let provider = self.provider.to_ascii_lowercase();
        if provider.contains("github") {
            json!({
                "id": 1,
                "login": "rullst-mock",
                "name": "Rullst Mock User",
                "email": "mock@example.invalid",
                "avatar_url": null
            })
        } else if provider.contains("discord") {
            json!({
                "id": "mock-user",
                "username": "Rullst Mock User",
                "email": "mock@example.invalid",
                "verified": true,
                "avatar": null
            })
        } else if provider.contains("facebook") {
            json!({
                "id": "mock-user",
                "name": "Rullst Mock User",
                "email": "mock@example.invalid",
                "picture": { "data": { "url": null } }
            })
        } else if provider.contains("microsoft") {
            json!({
                "id": "mock-user",
                "displayName": "Rullst Mock User",
                "mail": "mock@example.invalid"
            })
        } else if provider == "x" || provider.contains("xprovider") {
            json!({
                "data": {
                    "id": "mock-user",
                    "name": "Rullst Mock User",
                    "profile_image_url": null
                }
            })
        } else if provider.contains("cognito") {
            json!({
                "sub": "mock-user",
                "name": "Rullst Mock User",
                "username": "rullst-mock",
                "email": "mock@example.invalid",
                "picture": null
            })
        } else {
            json!({
                "sub": "mock-user",
                "name": "Rullst Mock User",
                "email": "mock@example.invalid",
                "email_verified": true,
                "picture": null
            })
        }
    }
}

#[async_trait]
impl HttpClient for OfflineHttpClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, ConnectError> {
        if !cfg!(any(test, feature = "mock")) {
            return Err(ConnectError::Offline(
                "mock credentials are network-free but functional mock identities require the 'mock' feature"
                    .to_string(),
            ));
        }

        let normalized_url = req.url.to_ascii_lowercase();
        let body = if normalized_url.contains("device/code") {
            json!({
                "device_code": "mock_device_code",
                "user_code": "MOCK-CODE",
                "verification_uri": "https://example.invalid/device",
                "expires_in": 900,
                "interval": 5
            })
        } else if req.method == "POST"
            && (normalized_url.contains("token") || normalized_url.contains("access_token"))
        {
            Self::token_response()
        } else if normalized_url.contains("jwks")
            || normalized_url.contains("certs")
            || normalized_url.ends_with("/keys")
        {
            json!({ "keys": [] })
        } else {
            self.user_response()
        };

        Ok(HttpResponse { status: 200, body })
    }
}

/// Transport used by deprecated infallible constructors after validation fails.
pub(crate) struct DisabledHttpClient {
    reason: String,
}

impl DisabledHttpClient {
    pub(crate) fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl HttpClient for DisabledHttpClient {
    async fn execute(&self, _req: HttpRequest) -> Result<HttpResponse, ConnectError> {
        Err(ConnectError::Offline(self.reason.clone()))
    }
}
