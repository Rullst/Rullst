pub mod client;
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

pub use error::ConnectError;

pub use provider::Provider;
pub use user::ConnectUser;

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
        name: &str,
        client_id: String,
        client_secret: secrecy::SecretString,
        redirect_url: String,
    ) -> Result<Box<dyn Provider>, crate::error::ConnectError> {
        let name = name.to_lowercase();
        let provider: Box<dyn Provider> = match name.as_str() {
            "github" => Box::new(crate::providers::GithubProvider::new(
                client_id,
                client_secret,
                redirect_url,
            )),
            "google" => Box::new(crate::providers::GoogleProvider::new(
                client_id,
                client_secret,
                redirect_url,
            )),
            "facebook" => Box::new(crate::providers::FacebookProvider::new(
                client_id,
                client_secret,
                redirect_url,
            )),
            "discord" => Box::new(crate::providers::DiscordProvider::new(
                client_id,
                client_secret,
                redirect_url,
            )),
            "linkedin" => Box::new(crate::providers::LinkedinProvider::new(
                client_id,
                client_secret,
                redirect_url,
            )),
            "x" => Box::new(crate::providers::XProvider::new(
                client_id,
                client_secret,
                redirect_url,
            )),
            "microsoft" => Box::new(crate::providers::MicrosoftProvider::new(
                client_id,
                client_secret,
                redirect_url,
            )),
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
}
