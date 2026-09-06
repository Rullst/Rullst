use super::{TokenExchangeForm, fetch_access_token, fetch_refresh_token};
use crate::client::{HttpClient, HttpRequest, HttpResponse};

struct TokenFixture(serde_json::Value);

#[async_trait::async_trait]
impl HttpClient for TokenFixture {
    async fn execute(&self, _request: HttpRequest) -> Result<HttpResponse, crate::ConnectError> {
        Ok(HttpResponse {
            status: 200,
            body: self.0.clone(),
        })
    }
}

#[tokio::test]
async fn exchanges_reject_invalid_or_excessive_token_lifetimes() {
    for invalid in [
        serde_json::json!(-1),
        serde_json::json!(0),
        serde_json::json!(u64::MAX),
        serde_json::json!(366 * 86400u64 + 1),
        serde_json::json!("3600"),
        serde_json::json!(true),
        serde_json::json!(1.5),
    ] {
        let fixture =
            TokenFixture(serde_json::json!({"access_token":"fixture", "expires_in":invalid}));
        let form = TokenExchangeForm {
            client_id: "fixture",
            client_secret: None,
            code: "code",
            grant_type: Some("authorization_code"),
            redirect_uri: "https://app.example/callback",
            code_verifier: None,
        };
        assert!(
            fetch_access_token(&fixture, "https://provider.invalid/token", &form)
                .await
                .is_err(),
            "authorization code accepted lifetime {invalid}"
        );
        assert!(
            fetch_refresh_token(
                &fixture,
                "https://provider.invalid/token",
                "fixture",
                "secret",
                "refresh"
            )
            .await
            .is_err(),
            "refresh accepted lifetime {invalid}"
        );
    }
}

#[tokio::test]
async fn exchanges_preserve_absent_and_bounded_positive_lifetimes() {
    for lifetime in [None, Some(1u64), Some(366 * 86400)] {
        let fixture =
            TokenFixture(serde_json::json!({"access_token":"fixture", "expires_in":lifetime}));
        let token = fetch_refresh_token(
            &fixture,
            "https://provider.invalid/token",
            "fixture",
            "secret",
            "refresh",
        )
        .await
        .unwrap();
        assert_eq!(token.expires_in, lifetime);
    }
}
