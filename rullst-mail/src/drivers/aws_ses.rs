// src/drivers/aws_ses.rs — AWS SES v2 mock, proxy and native transport boundary.

use super::traits::MailDriver;
use super::{DeliveryMode, credential_mode};
use crate::drivers::mock::{record_offline_delivery, validate_credential};
use crate::error::MailError;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};

#[cfg(feature = "aws-ses")]
mod native;

enum AwsSesTransport {
    FixtureOrProxy(SecretString),
    #[cfg(feature = "aws-ses")]
    Native(Box<native::NativeSesConfig>),
}

/// AWS SES v2 delivery with deterministic mock, explicit proxy and native modes.
///
/// [`Self::try_new`] retains the mock/proxy contract. Enable the `aws-ses`
/// feature and use [`Self::try_native`] or [`Self::from_native_config`] for
/// official AWS SDK delivery authenticated with Signature Version 4.
pub struct AwsSesDriver {
    region: String,
    transport: AwsSesTransport,
    endpoint_override: Option<String>,
}

impl std::fmt::Debug for AwsSesDriver {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AwsSesDriver")
            .field("region", &self.region)
            .field("transport", &self.transport_label())
            .field("endpoint_override", &self.endpoint_override)
            .finish()
    }
}

impl AwsSesDriver {
    /// Creates the deterministic mock or bearer-authenticated proxy adapter.
    ///
    /// Empty and `mock_*` tokens select the offline fixture. A real bearer
    /// token requires an explicit trusted endpoint override.
    pub fn try_new(
        region: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Result<Self, MailError> {
        let region = region.into();
        let auth_token = auth_token.into();
        validate_region(&region)?;
        validate_credential("AWS SES proxy authorization token", &auth_token)?;
        Ok(Self {
            region,
            transport: AwsSesTransport::FixtureOrProxy(SecretString::from(auth_token)),
            endpoint_override: None,
        })
    }

    /// Creates a native SES v2 driver from static or temporary AWS credentials.
    ///
    /// Prefer a rotating provider through [`Self::try_native_with_provider`] in
    /// long-running production services. Secrets are handed directly to the
    /// official AWS SDK credential type and are never formatted by Rullst.
    #[cfg(feature = "aws-ses")]
    pub fn try_native(
        region: impl Into<String>,
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
        session_token: Option<String>,
    ) -> Result<Self, MailError> {
        let region = region.into();
        let access_key_id = access_key_id.into();
        let secret_access_key = secret_access_key.into();
        validate_region(&region)?;
        validate_aws_credentials(&access_key_id, &secret_access_key, session_token.as_deref())?;
        let credentials = aws_sdk_sesv2::config::Credentials::new(
            access_key_id,
            secret_access_key,
            session_token,
            None,
            "rullst-mail-static",
        );
        Self::try_native_with_provider(region, credentials)
    }

    /// Creates a native SES driver with a caller-owned rotating credential provider.
    #[cfg(feature = "aws-ses")]
    pub fn try_native_with_provider<P>(
        region: impl Into<String>,
        provider: P,
    ) -> Result<Self, MailError>
    where
        P: aws_sdk_sesv2::config::ProvideCredentials + 'static,
    {
        let region = region.into();
        validate_region(&region)?;
        let config = native::config_with_provider(&region, provider);
        Ok(Self {
            region,
            transport: AwsSesTransport::Native(Box::new(native::NativeSesConfig::try_new(config)?)),
            endpoint_override: None,
        })
    }

    /// Wraps a caller-built official SES SDK config.
    ///
    /// This is the integration point for `aws-config` default chains, IAM role
    /// credentials, custom retry/timeout policy and refreshing providers.
    #[cfg(feature = "aws-ses")]
    pub fn from_native_config(config: aws_sdk_sesv2::Config) -> Result<Self, MailError> {
        let region = config
            .region()
            .map(|region| region.as_ref().to_string())
            .ok_or_else(|| {
                MailError::ConfigError("native AWS SES config requires a region".to_string())
            })?;
        validate_region(&region)?;
        Ok(Self {
            region,
            transport: AwsSesTransport::Native(Box::new(native::NativeSesConfig::try_new(config)?)),
            endpoint_override: None,
        })
    }

    /// Creates a mock/proxy driver without returning configuration errors.
    ///
    /// Prefer [`Self::try_new`]. Invalid configuration still fails closed in `send`.
    #[deprecated(since = "12.0.0", note = "use AwsSesDriver::try_new")]
    pub fn new(region: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            transport: AwsSesTransport::FixtureOrProxy(SecretString::from(auth_token.into())),
            endpoint_override: None,
        }
    }

    /// Sets a custom endpoint.
    ///
    /// Proxy mode expects the complete send URL. Native mode expects an SDK
    /// base endpoint and still appends `/v2/email/outbound-emails`.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint_override = Some(endpoint.into());
        self
    }

    /// Fallible endpoint override which validates HTTPS or explicit loopback HTTP.
    pub fn try_with_endpoint(mut self, endpoint: impl Into<String>) -> Result<Self, MailError> {
        let endpoint = endpoint.into();
        validate_endpoint(&endpoint)?;
        self.endpoint_override = Some(endpoint);
        Ok(self)
    }

    /// Returns the configured AWS region.
    pub fn region(&self) -> &str {
        &self.region
    }

    /// Returns whether delivery uses a real transport or the offline fallback.
    pub fn delivery_mode(&self) -> DeliveryMode {
        match &self.transport {
            AwsSesTransport::FixtureOrProxy(token) => credential_mode(token.expose_secret()),
            #[cfg(feature = "aws-ses")]
            AwsSesTransport::Native(_) => DeliveryMode::Real,
        }
    }

    /// Resolves the visible endpoint URL or base override.
    pub fn endpoint(&self) -> String {
        self.endpoint_override.clone().unwrap_or_else(|| {
            format!(
                "https://email.{}.amazonaws.com/v2/email/outbound-emails",
                self.region
            )
        })
    }

    fn transport_label(&self) -> &'static str {
        match &self.transport {
            AwsSesTransport::FixtureOrProxy(token)
                if credential_mode(token.expose_secret()) == DeliveryMode::OfflineMock =>
            {
                "offline_mock"
            }
            AwsSesTransport::FixtureOrProxy(_) => "bearer_proxy",
            #[cfg(feature = "aws-ses")]
            AwsSesTransport::Native(_) => "native_sigv4",
        }
    }
}

#[async_trait]
impl MailDriver for AwsSesDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        validate_region(&self.region)?;
        if let Some(endpoint) = self.endpoint_override.as_deref() {
            validate_endpoint(endpoint)?;
        }
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();

        match &self.transport {
            AwsSesTransport::FixtureOrProxy(token) => {
                validate_credential("AWS SES proxy authorization token", token.expose_secret())?;
                if self.delivery_mode() == DeliveryMode::OfflineMock {
                    return record_offline_delivery("aws_ses", message);
                }
                self.send_through_proxy(token, message).await
            }
            #[cfg(feature = "aws-ses")]
            AwsSesTransport::Native(config) => {
                DeliveryPipeline::require_due("AWS SES", message)?;
                config
                    .send(self.endpoint_override.as_deref(), message)
                    .await
            }
        }
    }
}

impl AwsSesDriver {
    async fn send_through_proxy(
        &self,
        token: &SecretString,
        message: &Message,
    ) -> Result<(), MailError> {
        DeliveryPipeline::require_due("AWS SES proxy", message)?;
        let endpoint = self.endpoint_override.as_deref().ok_or_else(|| {
            MailError::ConfigError(
                "AWS SES bearer mode requires an explicit trusted proxy endpoint; enable `aws-ses` and use a native constructor for direct SES delivery"
                    .to_string(),
            )
        })?;

        let client = super::http::client()?;
        let payload = proxy_payload(message);
        let response = client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .bearer_auth(token.expose_secret())
            .json(&payload)
            .send()
            .await
            .map_err(|_| MailError::transport("aws_ses_proxy", "request failed before response"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(crate::error::provider_http_error("aws_ses_proxy", response).await)
        }
    }
}

fn proxy_payload(message: &Message) -> serde_json::Value {
    let mut headers = Vec::new();
    if let Some(unsubscribe) = message.list_unsubscribe_header() {
        headers.push(serde_json::json!({"Name": "List-Unsubscribe", "Value": unsubscribe}));
        if message.unsubscribe_url.is_some() {
            headers.push(serde_json::json!({
                "Name": "List-Unsubscribe-Post",
                "Value": "List-Unsubscribe=One-Click"
            }));
        }
    }

    let mut body = serde_json::Map::new();
    if let Some(html) = &message.body_html {
        body.insert(
            "Html".to_string(),
            serde_json::json!({"Data": html, "Charset": "UTF-8"}),
        );
    }
    if let Some(text) = &message.body_text {
        body.insert(
            "Text".to_string(),
            serde_json::json!({"Data": text, "Charset": "UTF-8"}),
        );
    }
    let mut simple = serde_json::json!({
        "Subject": {"Data": message.subject, "Charset": "UTF-8"},
        "Body": body
    });
    if !headers.is_empty() {
        simple["Headers"] = serde_json::json!(headers);
    }
    serde_json::json!({
        "FromEmailAddress": message.from.as_deref().unwrap_or("noreply@rullst.dev"),
        "Destination": {"ToAddresses": [message.to]},
        "Content": {"Simple": simple}
    })
}

fn validate_region(region: &str) -> Result<(), MailError> {
    if region.is_empty()
        || region.len() > 64
        || !region
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(MailError::ConfigError(
            "AWS SES region must be 1-64 ASCII letters, digits, or '-'".to_string(),
        ));
    }
    Ok(())
}

#[cfg(feature = "aws-ses")]
fn validate_aws_credentials(
    access_key_id: &str,
    secret_access_key: &str,
    session_token: Option<&str>,
) -> Result<(), MailError> {
    validate_credential("AWS access key ID", access_key_id)?;
    validate_credential("AWS secret access key", secret_access_key)?;
    if access_key_id.trim().is_empty() || secret_access_key.trim().is_empty() {
        return Err(MailError::ConfigError(
            "native AWS SES credentials require non-empty access and secret keys".to_string(),
        ));
    }
    if let Some(token) = session_token {
        validate_credential("AWS session token", token)?;
        if token.trim().is_empty() {
            return Err(MailError::ConfigError(
                "AWS session token cannot be empty when supplied".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_endpoint(endpoint: &str) -> Result<(), MailError> {
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| MailError::ConfigError("AWS SES endpoint is not a valid URL".to_string()))?;
    let is_loopback_http = url.scheme() == "http"
        && url
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1"));
    if (url.scheme() == "https" || is_loopback_http) && url.host_str().is_some() {
        if !url.username().is_empty() || url.password().is_some() {
            return Err(MailError::ConfigError(
                "AWS SES endpoint must not contain embedded credentials".to_string(),
            ));
        }
        Ok(())
    } else {
        Err(MailError::ConfigError(
            "AWS SES endpoint must use HTTPS, except for explicit loopback HTTP".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn direct_bearer_mode_fails_before_unsigned_aws_request() {
        let driver = AwsSesDriver::try_new("us-east-1", "real-looking-secret")
            .expect("structurally valid configuration");
        let message = Message::new()
            .to("user@example.com")
            .from("sender@example.com")
            .subject("subject")
            .text("body");
        let error = driver
            .send(&message)
            .await
            .expect_err("native mode is explicit");
        assert!(matches!(error, MailError::ConfigError(_)));
    }

    #[test]
    fn debug_output_redacts_proxy_token() {
        let driver = AwsSesDriver::try_new("us-east-1", "provider-secret")
            .expect("valid proxy configuration");
        let output = format!("{driver:?}");
        assert!(!output.contains("provider-secret"));
        assert!(output.contains("bearer_proxy"));
    }

    #[test]
    fn endpoint_requires_https_or_loopback() {
        assert!(validate_endpoint("https://mail-proxy.example.com/send").is_ok());
        assert!(validate_endpoint("http://127.0.0.1:3000").is_ok());
        assert!(validate_endpoint("http://mail-proxy.example.com/send").is_err());
        assert!(validate_endpoint("https://user:secret@example.com/send").is_err());
    }
}

#[cfg(test)]
#[path = "aws_ses_tests.rs"]
mod contract_tests;
