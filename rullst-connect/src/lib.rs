pub mod client;
pub mod configuration;
pub mod error;
#[cfg(any(
    feature = "axum",
    feature = "actix",
    feature = "leptos",
    feature = "rullst"
))]
pub mod extractors;
#[macro_use]
pub mod macros;
pub mod mock_idp;
pub mod pkce;
pub mod prelude;
pub mod provider;
pub mod providers;
pub mod user;

pub use configuration::CredentialMode;
pub use error::ConnectError;

pub use provider::Provider;
pub use user::{ConnectUser, UniversalProfile};

/// The main entry point for the rullst-connect library.
pub struct Connect;

impl Connect {
    /// Factory method to dynamically instantiate an OAuth provider by name.
    ///
    /// Available providers (case-insensitive):
    /// "github", "google", "facebook", "gitlab", "discord", "linkedin", "x", "microsoft"
    ///
    /// Note: Providers requiring specialized configuration (like Apple, Auth0, Cognito, and Okta)
    /// must be instantiated manually.
    pub fn driver(
        name: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: secrecy::SecretString,
        redirect_url: impl Into<String>,
    ) -> Result<Box<dyn Provider>, crate::error::ConnectError> {
        let name = name.into().to_lowercase();
        let client_id = client_id.into();
        let redirect_url = redirect_url.into();
        let provider: Box<dyn Provider> = match name.as_str() {
            "github" => Box::new(crate::providers::GithubProvider::try_new(
                client_id,
                client_secret,
                redirect_url,
            )?),
            "google" => Box::new(crate::providers::GoogleProvider::try_new(
                client_id,
                client_secret,
                redirect_url,
            )?),
            "facebook" => Box::new(crate::providers::FacebookProvider::try_new(
                client_id,
                client_secret,
                redirect_url,
            )?),
            "discord" => Box::new(crate::providers::DiscordProvider::try_new(
                client_id,
                client_secret,
                redirect_url,
            )?),
            "linkedin" => Box::new(crate::providers::LinkedinProvider::try_new(
                client_id,
                client_secret,
                redirect_url,
            )?),
            "x" => Box::new(crate::providers::XProvider::try_new(
                client_id,
                client_secret,
                redirect_url,
            )?),
            "microsoft" => Box::new(crate::providers::MicrosoftProvider::try_new(
                client_id,
                client_secret,
                redirect_url,
            )?),
            "apple" | "auth0" | "cognito" | "oidc" => {
                return Err(crate::error::ConnectError::Provider(format!(
                    "Provider '{}' requires custom configuration (domain or key_id) and cannot be instantiated via the generic driver factory. Please instantiate it directly.",
                    name
                )));
            }
            _ => {
                return Err(crate::error::ConnectError::Provider(format!(
                    "Unknown provider: {}",
                    name
                )));
            }
        };
        Ok(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_factory() {
        let github = Connect::driver(
            "github",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(github.is_ok());

        let apple = Connect::driver(
            "apple",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(
            matches!(apple, Err(crate::error::ConnectError::Provider(ref msg)) if msg.contains("requires custom configuration"))
        );

        let unknown = Connect::driver(
            "unknown",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(unknown.is_err());

        // Test all supported factory providers
        let google = Connect::driver(
            "google",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(google.is_ok());

        let facebook = Connect::driver(
            "facebook",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(facebook.is_ok());

        let discord = Connect::driver(
            "discord",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(discord.is_ok());

        let linkedin = Connect::driver(
            "linkedin",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(linkedin.is_ok());

        let x = Connect::driver(
            "x",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(x.is_ok());

        let microsoft = Connect::driver(
            "microsoft",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(microsoft.is_ok());

        // Test all unsupported factory providers
        let auth0 = Connect::driver(
            "auth0",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(
            matches!(auth0, Err(crate::error::ConnectError::Provider(ref msg)) if msg.contains("requires custom configuration"))
        );

        let cognito = Connect::driver(
            "cognito",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(
            matches!(cognito, Err(crate::error::ConnectError::Provider(ref msg)) if msg.contains("requires custom configuration"))
        );

        let oidc = Connect::driver(
            "oidc",
            "id".to_string(),
            secrecy::SecretString::from("secret".to_string()),
            "https://url".to_string(),
        );
        assert!(
            matches!(oidc, Err(crate::error::ConnectError::Provider(ref msg)) if msg.contains("requires custom configuration"))
        );
    }

    #[tokio::test]
    async fn empty_credentials_use_a_deterministic_offline_driver() {
        let provider = Connect::driver(
            "github",
            "",
            secrecy::SecretString::from("".to_string()),
            "https://app.example/callback",
        )
        .expect("mock driver");

        let user = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "offline-code",
                ..Default::default()
            })
            .await
            .expect("offline user");
        assert_eq!(user.id, "1");
        assert_eq!(user.email.as_deref(), Some("mock@example.invalid"));
        assert!(
            provider
                .redirect_url()
                .starts_with("https://example.invalid/")
        );
    }

    #[test]
    fn driver_rejects_an_insecure_redirect_without_panicking() {
        let result = Connect::driver(
            "github",
            "client",
            secrecy::SecretString::from("secret".to_string()),
            "http://localhost.evil/callback",
        );
        assert!(matches!(
            result,
            Err(ConnectError::InvalidConfiguration {
                field: "redirect_url",
                ..
            })
        ));
    }
}
