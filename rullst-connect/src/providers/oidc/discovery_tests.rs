//! Discovery accepts metadata that omits the RECOMMENDED userinfo endpoint.

use super::discovery::OidcProvider;
use crate::client::{HttpClient, HttpRequest, HttpResponse};
use crate::error::ConnectError;
use crate::provider::Provider;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

struct MetadataClient {
    metadata: Value,
    requested: Mutex<Vec<String>>,
}

impl MetadataClient {
    fn new(metadata: Value) -> Arc<Self> {
        Arc::new(Self {
            metadata,
            requested: Mutex::new(Vec::new()),
        })
    }

    fn requested(&self) -> Vec<String> {
        self.requested
            .lock()
            .map(|urls| urls.clone())
            .unwrap_or_default()
    }
}

#[async_trait]
impl HttpClient for MetadataClient {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ConnectError> {
        if let Ok(mut urls) = self.requested.lock() {
            urls.push(request.url.clone());
        }
        let body = if request.url.contains("openid-configuration") {
            self.metadata.clone()
        } else {
            json!({"access_token": "fixture-access", "expires_in": 60})
        };
        Ok(HttpResponse { status: 200, body })
    }
}

fn metadata_without_userinfo() -> Value {
    json!({
        "issuer": "https://issuer.example",
        "authorization_endpoint": "https://issuer.example/authorize",
        "token_endpoint": "https://issuer.example/token",
        "jwks_uri": "https://issuer.example/jwks"
    })
}

async fn discover(client: Arc<MetadataClient>) -> Result<OidcProvider, ConnectError> {
    OidcProvider::discover_with_client(
        "https://issuer.example",
        "client_id",
        "client_secret",
        "https://app.example/callback",
        client,
    )
    .await
}

fn is_missing_userinfo(error: &ConnectError) -> bool {
    matches!(
        error,
        ConnectError::InvalidConfiguration {
            field: "userinfo_endpoint",
            ..
        }
    )
}

#[tokio::test]
async fn discovery_without_userinfo_endpoint_succeeds_and_userinfo_fails_typed() {
    for metadata in [metadata_without_userinfo(), {
        let mut metadata = metadata_without_userinfo();
        metadata["userinfo_endpoint"] = Value::Null;
        metadata
    }] {
        let client = MetadataClient::new(metadata);
        let provider = discover(client.clone()).await.unwrap();
        assert!(provider.userinfo_endpoint.is_empty());

        let direct = provider.get_user_from_token("access").await.unwrap_err();
        assert!(is_missing_userinfo(&direct));

        // A code exchange that returns no id_token needs userinfo.
        let exchange = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "code",
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(is_missing_userinfo(&exchange));
        assert!(!client.requested().iter().any(|url| url.is_empty()));
    }
}

#[tokio::test]
async fn a_published_userinfo_endpoint_is_still_validated() {
    for invalid in [
        json!(""),
        json!(7),
        json!("http://attacker.example/userinfo"),
    ] {
        let mut metadata = metadata_without_userinfo();
        metadata["userinfo_endpoint"] = invalid;
        assert!(discover(MetadataClient::new(metadata)).await.is_err());
    }
}
