use async_trait::async_trait;
use rullst_connect::client::{HttpClient, HttpRequest, HttpResponse};
use rullst_connect::prelude::*;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

const APPLE_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----\n\
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgkdn4ngP0MJj/+G/Z\n\
0FgfmUYbc26Oidgl0NZoUXoMm6KhRANCAARcJ2gzcG1e8qufjKrOWQSmC4OoQkAU\n\
k/Tz7c8S43tqF0VK/mNC462881k2cryVtuV5FkH1XoPACJzJUQ5igUZV\n\
-----END PRIVATE KEY-----";

struct CaptureClient {
    status: u16,
    request: Arc<Mutex<Option<HttpRequest>>>,
}

#[async_trait]
impl HttpClient for CaptureClient {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ConnectError> {
        *self.request.lock().await = Some(request);
        Ok(HttpResponse {
            status: self.status,
            body: if self.status >= 400 {
                json!({"error_description": "provider-secret-marker"})
            } else {
                json!({})
            },
        })
    }
}

fn capture_client(status: u16) -> (Arc<dyn HttpClient>, Arc<Mutex<Option<HttpRequest>>>) {
    let request = Arc::new(Mutex::new(None));
    let client = Arc::new(CaptureClient {
        status,
        request: request.clone(),
    });
    (client, request)
}

async fn take_request(request: &Arc<Mutex<Option<HttpRequest>>>) -> HttpRequest {
    request
        .lock()
        .await
        .take()
        .expect("revocation request was captured")
}

#[tokio::test]
async fn google_revokes_both_token_kinds_without_a_type_hint() {
    let (client, request) = capture_client(200);
    let provider = GoogleProvider::try_new(
        "google-client",
        SecretString::from("google-secret".to_string()),
        "https://app.example/callback",
    )
    .expect("valid provider")
    .with_http_client(client);

    for (token, revoke_refresh) in [
        ("google-access-marker", false),
        ("google-refresh-marker", true),
    ] {
        if revoke_refresh {
            provider
                .revoke_refresh_token(token)
                .await
                .expect("refresh token revoked");
        } else {
            provider
                .revoke_token(token)
                .await
                .expect("access token revoked");
        }
        let captured = take_request(&request).await;
        assert_eq!(captured.method, "POST");
        assert_eq!(captured.url, "https://oauth2.googleapis.com/revoke");
        let expected_form = format!("token={token}");
        assert_eq!(captured.form.as_deref(), Some(expected_form.as_str()));
        assert!(captured.basic_auth.is_none());
    }
}

#[tokio::test]
async fn discord_revokes_access_and_refresh_tokens_with_basic_auth_and_hints() {
    let (client, request) = capture_client(200);
    let provider = DiscordProvider::try_new(
        "discord-client",
        SecretString::from("discord-secret".to_string()),
        "https://app.example/callback",
    )
    .expect("valid provider")
    .with_http_client(client);

    provider
        .revoke_token("access-marker")
        .await
        .expect("access token revoked");
    let access = take_request(&request).await;
    assert_eq!(access.method, "POST");
    assert_eq!(access.url, "https://discord.com/api/oauth2/token/revoke");
    assert_eq!(
        access.basic_auth,
        Some((
            "discord-client".to_string(),
            Some("discord-secret".to_string())
        ))
    );
    assert_eq!(
        access.form.as_deref(),
        Some("token=access-marker&token_type_hint=access_token")
    );

    provider
        .revoke_refresh_token("refresh-marker")
        .await
        .expect("refresh token revoked");
    let refresh = take_request(&request).await;
    assert_eq!(
        refresh.form.as_deref(),
        Some("token=refresh-marker&token_type_hint=refresh_token")
    );
}

#[tokio::test]
async fn github_revokes_only_access_tokens_with_an_encoded_path_and_json_body() {
    let (client, request) = capture_client(204);
    let provider = GithubProvider::try_new(
        "client/id",
        SecretString::from("github-secret".to_string()),
        "https://app.example/callback",
    )
    .expect("valid provider")
    .with_http_client(client);

    provider
        .revoke_token("github-access-marker")
        .await
        .expect("access token revoked");
    let captured = take_request(&request).await;
    assert_eq!(captured.method, "DELETE");
    assert_eq!(
        captured.url,
        "https://api.github.com/applications/client%2Fid/token"
    );
    assert_eq!(
        captured.json,
        Some(json!({"access_token": "github-access-marker"}))
    );
    assert_eq!(
        captured.basic_auth,
        Some(("client/id".to_string(), Some("github-secret".to_string())))
    );
    assert_eq!(
        captured
            .headers
            .get("User-Agent")
            .and_then(|value| value.to_str().ok()),
        Some("rullst-connect")
    );

    assert!(matches!(
        provider.revoke_refresh_token("refresh-marker").await,
        Err(ConnectError::Token(message)) if message.contains("refresh_token")
    ));
    assert!(request.lock().await.is_none());
}

#[tokio::test]
async fn auth0_and_cognito_require_refresh_tokens_and_use_their_documented_auth_modes() {
    let (auth0_client, auth0_request) = capture_client(200);
    let auth0 = Auth0Provider::try_new(
        "auth0-client",
        SecretString::from("auth0-secret".to_string()),
        "https://app.example/callback",
        "tenant.auth0.com",
    )
    .expect("valid Auth0 provider")
    .with_http_client(auth0_client);
    assert!(auth0.revoke_token("access-marker").await.is_err());
    auth0
        .revoke_refresh_token("auth0-refresh-marker")
        .await
        .expect("Auth0 refresh token revoked");
    let auth0 = take_request(&auth0_request).await;
    assert_eq!(auth0.url, "https://tenant.auth0.com/oauth/revoke");
    assert_eq!(
        auth0.form.as_deref(),
        Some("client_id=auth0-client&client_secret=auth0-secret&token=auth0-refresh-marker")
    );
    assert!(auth0.basic_auth.is_none());

    let (cognito_client, cognito_request) = capture_client(200);
    let cognito = CognitoProvider::try_new(
        "cognito-client",
        SecretString::from("cognito-secret".to_string()),
        "https://app.example/callback",
        "https://tenant.auth.us-east-1.amazoncognito.com",
    )
    .expect("valid Cognito provider")
    .with_http_client(cognito_client);
    assert!(cognito.revoke_token("access-marker").await.is_err());
    cognito
        .revoke_refresh_token("cognito-refresh-marker")
        .await
        .expect("Cognito refresh token revoked");
    let cognito = take_request(&cognito_request).await;
    assert_eq!(
        cognito.url,
        "https://tenant.auth.us-east-1.amazoncognito.com/oauth2/revoke"
    );
    assert_eq!(
        cognito.form.as_deref(),
        Some("token=cognito-refresh-marker")
    );
    assert_eq!(
        cognito.basic_auth,
        Some((
            "cognito-client".to_string(),
            Some("cognito-secret".to_string())
        ))
    );
}

#[tokio::test]
async fn apple_generates_a_short_lived_client_secret_and_sends_the_token_hint() {
    let (client, request) = capture_client(200);
    let provider = AppleProvider::try_new(
        "com.example.web",
        "TEAM123",
        "KEY123",
        APPLE_PRIVATE_KEY,
        "https://app.example/callback",
    )
    .expect("valid Apple provider")
    .with_http_client(client);

    provider
        .revoke_refresh_token("apple-refresh-marker")
        .await
        .expect("Apple refresh token revoked");
    let captured = take_request(&request).await;
    assert_eq!(captured.url, "https://appleid.apple.com/auth/revoke");
    let form: std::collections::HashMap<_, _> =
        url::form_urlencoded::parse(captured.form.as_deref().expect("form").as_bytes())
            .into_owned()
            .collect();
    assert_eq!(
        form.get("client_id").map(String::as_str),
        Some("com.example.web")
    );
    assert_eq!(
        form.get("token").map(String::as_str),
        Some("apple-refresh-marker")
    );
    assert_eq!(
        form.get("token_type_hint").map(String::as_str),
        Some("refresh_token")
    );
    assert_eq!(
        form.get("client_secret")
            .map(|secret| secret.split('.').count()),
        Some(3)
    );
}

#[tokio::test]
async fn revocation_rejects_malformed_tokens_before_transport_and_maps_provider_errors() {
    let (client, request) = capture_client(200);
    let provider = DiscordProvider::try_new(
        "discord-client",
        SecretString::from("discord-secret".to_string()),
        "https://app.example/callback",
    )
    .expect("valid provider")
    .with_http_client(client);
    assert!(matches!(
        provider.revoke_token(" padded-token ").await,
        Err(ConnectError::Token(_))
    ));
    assert!(request.lock().await.is_none());

    let (client, _) = capture_client(401);
    let provider = DiscordProvider::try_new(
        "discord-client",
        SecretString::from("discord-secret".to_string()),
        "https://app.example/callback",
    )
    .expect("valid provider")
    .with_http_client(client);
    let error = provider
        .revoke_token("access-marker")
        .await
        .expect_err("provider rejection");
    assert!(matches!(error, ConnectError::ProviderApiError { .. }));
    assert!(!error.to_string().contains("provider-secret-marker"));

    let (client, _) = capture_client(302);
    let provider = DiscordProvider::try_new(
        "discord-client",
        SecretString::from("discord-secret".to_string()),
        "https://app.example/callback",
    )
    .expect("valid provider")
    .with_http_client(client);
    assert!(matches!(
        provider.revoke_token("access-marker").await,
        Err(ConnectError::ProviderApiError { .. })
    ));
}

#[tokio::test]
async fn offline_credentials_revoke_without_installing_a_live_transport() {
    let provider = Auth0Provider::try_new(
        "mock_client",
        SecretString::from("mock_secret".to_string()),
        "https://app.example/callback",
        "tenant.auth0.com",
    )
    .expect("offline provider");
    provider
        .revoke_refresh_token("offline-refresh-token")
        .await
        .expect("deterministic offline revocation");
}
