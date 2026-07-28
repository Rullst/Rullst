use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;

/// A mock provider specifically designed for testing and TDD.
/// It returns a pre-configured `ConnectUser` without performing any HTTP requests.
pub struct MockProvider {
    mocked_user: ConnectUser,
    mocked_url: String,
    expect_revoke_success: bool,
}

impl MockProvider {
    /// Creates a new `MockProvider` with a static user and login URL.
    pub fn new(user: ConnectUser, url: String) -> Self {
        Self {
            mocked_user: user,
            mocked_url: url,
            expect_revoke_success: true,
        }
    }

    /// Sets whether the `revoke_token` method should succeed or fail.
    pub fn with_revoke_success(mut self, success: bool) -> Self {
        self.expect_revoke_success = success;
        self
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn token_url(&self) -> String {
        "https://mock.provider/token".to_string()
    }

    fn redirect_url(&self) -> String {
        self.mocked_url.clone()
    }

    async fn get_user(
        &self,
        _params: crate::provider::ExchangeParams<'_>,
    ) -> Result<crate::user::ConnectUser, crate::error::ConnectError> {
        Ok(self.mocked_user.clone())
    }

    async fn get_user_from_token(
        &self,
        _access_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        Ok(self.mocked_user.clone())
    }

    async fn revoke_token(&self, _token: &str) -> Result<(), crate::error::ConnectError> {
        if self.expect_revoke_success {
            Ok(())
        } else {
            Err(crate::error::ConnectError::Token(
                "Mocked revocation failure".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider_revoke_token() {
        let user = ConnectUser {
            id: "1".to_string(),
            name: "Test".to_string(),
            email: None,
            avatar_url: None,
            email_verified: None,
            raw_data: serde_json::json!({}),
            access_token: secrecy::SecretString::from("token".to_string()),
            refresh_token: None,
            expires_in: None,
        };
        let provider = MockProvider::new(user, "http://mock".to_string());

        // Default is success
        assert!(provider.revoke_token("token").await.is_ok());

        // Test with revoke success false
        let provider = provider.with_revoke_success(false);
        let res = provider.revoke_token("token").await;
        assert!(res.is_err());
        if let Err(crate::error::ConnectError::Token(msg)) = res {
            assert_eq!(msg, "Mocked revocation failure");
        } else {
            panic!("Expected ConnectError::Token");
        }
    }

    #[tokio::test]
    async fn test_mock_provider_basics() {
        let user = ConnectUser {
            id: "1".to_string(),
            name: "Test".to_string(),
            email: None,
            avatar_url: None,
            email_verified: None,
            raw_data: serde_json::json!({}),
            access_token: secrecy::SecretString::from("token".to_string()),
            refresh_token: None,
            expires_in: None,
        };
        let provider = MockProvider::new(user, "http://mock.redirect".to_string());

        assert_eq!(provider.token_url(), "https://mock.provider/token");
        assert_eq!(provider.redirect_url(), "http://mock.redirect");

        let fetched_user = provider
            .get_user(crate::provider::ExchangeParams::default())
            .await
            .unwrap();
        assert_eq!(fetched_user.id, "1");

        let fetched_user_token = provider.get_user_from_token("dummy_token").await.unwrap();
        assert_eq!(fetched_user_token.id, "1");
    }
}
