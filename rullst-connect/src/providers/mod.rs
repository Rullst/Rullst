pub mod apple;
pub mod auth0;
pub mod cognito;
pub mod discord;
pub mod facebook;
pub mod github;
pub mod google;
pub mod linkedin;
pub mod microsoft;
pub mod oidc;
pub mod x;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

pub use apple::AppleProvider;
pub use auth0::Auth0Provider;
pub use cognito::CognitoProvider;
pub use discord::DiscordProvider;
pub use facebook::FacebookProvider;
pub use github::GithubProvider;
pub use google::GoogleProvider;
pub use linkedin::LinkedinProvider;
pub use microsoft::MicrosoftProvider;
pub use oidc::OidcProvider;
pub use x::XProvider;

#[cfg(any(test, feature = "mock"))]
pub use mock::MockProvider;
