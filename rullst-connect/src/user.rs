use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a standardized user profile returned from any OAuth2 provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectUser {
    /// The unique identifier of the user in the provider's system.
    pub id: String,

    /// The full name or display name of the user.
    pub name: String,

    /// The email address of the user, if available and granted.
    pub email: Option<String>,

    /// Indicates whether the provider has verified the user's email address.
    pub email_verified: Option<bool>,

    /// The URL to the user's avatar/profile picture, if available.
    pub avatar_url: Option<String>,

    /// The raw JSON response received from the provider's user endpoint.
    /// Useful for extracting provider-specific fields not covered by this struct.
    pub raw_data: Value,

    /// The access token retrieved during the OAuth2 flow.
    #[serde(skip_serializing, deserialize_with = "secret_serde::deserialize")]
    pub access_token: secrecy::SecretString,

    /// The refresh token retrieved during the OAuth2 flow (if provided).
    #[serde(
        skip_serializing,
        default,
        deserialize_with = "opt_secret_serde::deserialize"
    )]
    pub refresh_token: Option<secrecy::SecretString>,

    /// The token expiration time in seconds from the time it was granted (if provided).
    pub expires_in: Option<u64>,
}

mod secret_serde {
    use secrecy::SecretString;
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(SecretString::from(s))
    }
}

mod opt_secret_serde {
    use secrecy::SecretString;
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SecretString>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        Ok(opt.map(SecretString::from))
    }
}

/// Provider-independent profile data that is safe to serialize.
///
/// Access and refresh tokens, expiry metadata and raw provider payloads are
/// deliberately excluded. Persist tokens only in a dedicated encrypted secret
/// store with an application-defined lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversalProfile {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub avatar_url: Option<String>,
}

impl ConnectUser {
    /// Returns the normalized, credential-free profile projection.
    pub fn universal_profile(&self) -> UniversalProfile {
        UniversalProfile::from(self)
    }
}

impl From<&ConnectUser> for UniversalProfile {
    fn from(user: &ConnectUser) -> Self {
        Self {
            id: user.id.clone(),
            name: user.name.clone(),
            email: user.email.clone(),
            email_verified: user.email_verified,
            avatar_url: user.avatar_url.clone(),
        }
    }
}

use async_trait::async_trait;

/// Helper trait to seamlessly integrate `ConnectUser` with databases and ORMs (like SQLx, Diesel, rullst-orm).
/// By implementing this trait on your custom database User model or repository, you can easily
/// save or update users directly from the OAuth profile.
#[async_trait]
pub trait IntoDatabaseUser<T> {
    /// Inserts or updates the user in the database based on the OAuth profile.
    /// Returns the database-specific User model or an error.
    async fn sync_from_oauth(profile: &ConnectUser) -> Result<T, crate::error::ConnectError>;
}

/// Represents the response from a device authorization request (RFC 8628).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceAuthorizationResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn connect_user_serialization_never_exposes_tokens() {
        let user = ConnectUser {
            id: "123".to_string(),
            name: "Test User".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: Some(true),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            raw_data: json!({"custom_field": "custom_value"}),
            access_token: secrecy::SecretString::from("access123".to_string()),
            refresh_token: Some(secrecy::SecretString::from("refresh123".to_string())),
            expires_in: Some(3600),
        };

        let serialized = serde_json::to_value(&user).unwrap();
        assert_eq!(serialized["id"], "123");
        assert!(serialized.get("access_token").is_none());
        assert!(serialized.get("refresh_token").is_none());
        assert!(!serialized.to_string().contains("access123"));
        assert!(!serialized.to_string().contains("refresh123"));

        let profile = user.universal_profile();
        let public_json = serde_json::to_value(&profile).unwrap();
        assert_eq!(public_json["email"], "test@example.com");
        assert!(public_json.get("raw_data").is_none());
        assert!(public_json.get("expires_in").is_none());
    }

    #[test]
    fn test_debug_and_clone() {
        let user = ConnectUser {
            id: "123".to_string(),
            name: "Test User".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: Some(true),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            raw_data: json!({"custom_field": "custom_value"}),
            access_token: secrecy::SecretString::from("access123".to_string()),
            refresh_token: Some(secrecy::SecretString::from("refresh123".to_string())),
            expires_in: Some(3600),
        };
        let _cloned = user.clone();
        let _debug = format!("{:?}", user);

        let device = DeviceAuthorizationResponse {
            device_code: "device".to_string(),
            user_code: "user".to_string(),
            verification_uri: "uri".to_string(),
            verification_uri_complete: Some("uri_complete".to_string()),
            expires_in: 3600,
            interval: Some(5),
        };
        let _cloned_device = device.clone();
        let _debug_device = format!("{:?}", device);
    }

    #[derive(Debug, serde::Deserialize)]
    struct TestOptSecret {
        #[serde(deserialize_with = "crate::user::opt_secret_serde::deserialize")]
        pub secret: Option<secrecy::SecretString>,
    }

    #[test]
    fn test_opt_secret_serde_deserialize() {
        use secrecy::ExposeSecret;

        // Test with a value
        let json_with_val = r#"{"secret": "my_super_secret"}"#;
        let parsed: TestOptSecret = serde_json::from_str(json_with_val).unwrap();
        assert!(parsed.secret.is_some());
        assert_eq!(parsed.secret.unwrap().expose_secret(), "my_super_secret");

        // Test with null
        let json_with_null = r#"{"secret": null}"#;
        let parsed_null: TestOptSecret = serde_json::from_str(json_with_null).unwrap();
        assert!(parsed_null.secret.is_none());
    }
}
