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
    #[serde(with = "secret_serde")]
    pub access_token: secrecy::SecretString,

    /// The refresh token retrieved during the OAuth2 flow (if provided).
    #[serde(with = "opt_secret_serde")]
    pub refresh_token: Option<secrecy::SecretString>,

    /// The token expiration time in seconds from the time it was granted (if provided).
    pub expires_in: Option<u64>,
}

mod secret_serde {
    use secrecy::{ExposeSecret, SecretString};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        secret.expose_secret().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(SecretString::from(s))
    }
}

mod opt_secret_serde {
    use secrecy::{ExposeSecret, SecretString};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(secret: &Option<SecretString>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match secret {
            Some(s) => s.expose_secret().serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SecretString>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        Ok(opt.map(SecretString::from))
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
    fn test_connect_user_serialization() {
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

        let serialized = serde_json::to_string(&user).unwrap();
        let deserialized: ConnectUser = serde_json::from_str(&serialized).unwrap();

        assert_eq!(user.id, deserialized.id);
        assert_eq!(user.name, deserialized.name);
        assert_eq!(user.email, deserialized.email);
        assert_eq!(user.email_verified, deserialized.email_verified);
        assert_eq!(user.avatar_url, deserialized.avatar_url);
        assert_eq!(user.raw_data, deserialized.raw_data);
        use secrecy::ExposeSecret;
        assert_eq!(
            user.access_token.expose_secret(),
            deserialized.access_token.expose_secret()
        );
        assert_eq!(
            user.refresh_token.as_ref().map(|s| s.expose_secret()),
            deserialized
                .refresh_token
                .as_ref()
                .map(|s| s.expose_secret())
        );
        assert_eq!(user.expires_in, deserialized.expires_in);
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

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct TestOptSecret {
        #[serde(with = "crate::user::opt_secret_serde")]
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
