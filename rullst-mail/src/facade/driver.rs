//! The driver the `Mail` facade builds from its settings, reused across
//! messages while those settings stay the same.
//!
//! Rebuilding the driver for every message discarded per-driver state such
//! as the `AzureManagedIdentity` token cache, the native SES SDK client and
//! HTTP connection pools. The facade still reads its settings on every call,
//! so a changed setting takes effect on the next message.

use super::settings::MailSettings;
use crate::drivers::*;
use sha2::{Digest, Sha256};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// A built driver and the SHA-256 digest of the settings it was built from,
/// so the key holds no copy of a credential.
type CachedDriver = ([u8; 32], Arc<dyn MailDriver>);

/// The last driver built from the facade settings.
static CONFIGURED_DRIVER: Mutex<Option<CachedDriver>> = Mutex::new(None);

/// Every value one configured driver is built from. It is never formatted.
#[derive(Hash)]
pub(super) enum DriverSpec {
    Log,
    Memory,
    #[cfg(feature = "mail-smtp")]
    Smtp {
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    },
    #[cfg(not(feature = "mail-smtp"))]
    Smtp,
    Resend(String),
    SendPulse(String),
    Mailjet {
        api_key: String,
        secret_key: String,
        sandbox: bool,
    },
    Mailtrap {
        api_token: String,
        sandbox_id: Option<u64>,
    },
    SendGrid(String),
    Postmark {
        server_token: String,
        message_stream: Option<String>,
    },
    /// `identity` holds the managed-identity environment for a real endpoint.
    AzureAcs {
        endpoint: String,
        identity: Option<[Option<String>; 3]>,
    },
    Ses {
        region: String,
        endpoint: Option<String>,
        credentials: SesCredentials,
    },
}

#[derive(Hash)]
pub(super) enum SesCredentials {
    #[cfg(feature = "aws-ses")]
    Native {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    },
    Proxy(String),
}

/// Feeds a derived `Hash` into SHA-256. Strings hash with a `0xff`
/// terminator and enums with their discriminant, so the encoding is
/// unambiguous.
struct DigestHasher(Sha256);

impl Hasher for DigestHasher {
    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    fn finish(&self) -> u64 {
        0
    }
}

impl DriverSpec {
    /// Reads the settings of the selected driver, failing closed on invalid ones.
    pub(super) fn from_settings(settings: &MailSettings) -> Result<Self, MailError> {
        let driver_name = settings.driver_name()?;
        let value = |name: &str| settings.value(name);
        let required = |name: &str| -> Result<String, MailError> {
            Ok(settings.value(name)?.unwrap_or_default())
        };
        Ok(match driver_name.as_str() {
            "log" => Self::Log,
            "memory" => Self::Memory,
            #[cfg(feature = "mail-smtp")]
            "smtp" => Self::Smtp {
                host: value("MAIL_HOST")?.unwrap_or_else(|| "127.0.0.1".to_string()),
                port: smtp_port(value("MAIL_PORT")?)?,
                username: value("MAIL_USERNAME")?,
                password: value("MAIL_PASSWORD")?,
            },
            #[cfg(not(feature = "mail-smtp"))]
            "smtp" => Self::Smtp,
            "resend" => Self::Resend(required("RESEND_API_KEY")?),
            "sendpulse" => Self::SendPulse(required("SENDPULSE_API_KEY")?),
            "mailjet" | "mailjet-sandbox" => Self::Mailjet {
                api_key: required("MAILJET_API_KEY")?,
                secret_key: required("MAILJET_SECRET_KEY")?,
                sandbox: driver_name == "mailjet-sandbox",
            },
            "mailtrap" => Self::Mailtrap {
                api_token: required("MAILTRAP_API_TOKEN")?,
                sandbox_id: None,
            },
            "mailtrap-sandbox" => {
                let id = value("MAILTRAP_SANDBOX_ID")?
                    .and_then(|v| v.parse::<u64>().ok())
                    .ok_or_else(|| {
                        MailError::ConfigError(
                            "MAILTRAP_SANDBOX_ID must be a positive integer".into(),
                        )
                    })?;
                Self::Mailtrap {
                    api_token: required("MAILTRAP_API_TOKEN")?,
                    sandbox_id: Some(id),
                }
            }
            "sendgrid" => Self::SendGrid(required("SENDGRID_API_KEY")?),
            "postmark" => Self::Postmark {
                server_token: match value("POSTMARK_SERVER_TOKEN")? {
                    Some(token) => token,
                    None => required("POSTMARK_API_KEY")?,
                },
                message_stream: value("POSTMARK_MESSAGE_STREAM")?,
            },
            "azure-acs" => {
                let endpoint = required("AZURE_COMMUNICATION_EMAIL_ENDPOINT")?;
                // `AzureManagedIdentity::from_environment` reads these from
                // the process environment, so they are part of the key.
                let identity =
                    (!endpoint.is_empty() && !endpoint.starts_with("mock_")).then(|| {
                        ["IDENTITY_ENDPOINT", "IDENTITY_HEADER", "AZURE_CLIENT_ID"]
                            .map(|name| std::env::var(name).ok())
                    });
                Self::AzureAcs { endpoint, identity }
            }
            "ses" | "aws_ses" => Self::Ses {
                region: value("AWS_REGION")?.unwrap_or_else(|| "us-east-1".to_string()),
                endpoint: value("AWS_SES_ENDPOINT")?,
                credentials: ses_credentials(settings)?,
            },
            other => {
                return Err(MailError::ConfigError(format!(
                    "Unknown mail driver: {}",
                    other
                )));
            }
        })
    }

    /// Builds the driver; constructors validate credentials and endpoints.
    #[cfg_attr(mutants, mutants::skip)]
    pub(super) fn build(self) -> Result<Box<dyn MailDriver>, MailError> {
        Ok(match self {
            Self::Log => Box::new(LogDriver),
            Self::Memory => Box::new(MemoryDriver::new()),
            #[cfg(feature = "mail-smtp")]
            Self::Smtp {
                host,
                port,
                username,
                password,
            } => Box::new(SmtpDriver::try_new(host, port, username, password)?),
            #[cfg(not(feature = "mail-smtp"))]
            Self::Smtp => Box::new(SmtpDriver),
            Self::Resend(api_key) => Box::new(ResendDriver::try_new(api_key)?),
            Self::SendPulse(api_key) => Box::new(SendPulseDriver::try_new(api_key)?),
            Self::Mailjet {
                api_key,
                secret_key,
                sandbox,
            } => {
                let driver = MailjetDriver::try_new(api_key, secret_key)?;
                Box::new(if sandbox {
                    driver.with_sandbox()
                } else {
                    driver
                })
            }
            Self::Mailtrap {
                api_token,
                sandbox_id: None,
            } => Box::new(MailtrapDriver::try_new(api_token)?),
            Self::Mailtrap {
                api_token,
                sandbox_id: Some(id),
            } => Box::new(MailtrapDriver::sandbox(api_token, id)?),
            Self::SendGrid(api_key) => Box::new(SendGridDriver::try_new(api_key)?),
            Self::Postmark {
                server_token,
                message_stream,
            } => {
                let mut driver = PostmarkDriver::try_new(server_token)?;
                if let Some(stream) = message_stream {
                    driver = driver.with_message_stream(stream);
                }
                Box::new(driver)
            }
            Self::AzureAcs {
                endpoint,
                identity: None,
            } => Box::new(AzureCommunicationDriver::new(
                endpoint,
                StaticAzureMailCredential::new("mock_azure", 0)?,
            )?),
            Self::AzureAcs {
                endpoint,
                identity: Some(_),
            } => Box::new(AzureCommunicationDriver::new(
                endpoint,
                AzureManagedIdentity::from_environment()?,
            )?),
            Self::Ses {
                region,
                endpoint,
                credentials,
            } => {
                let mut driver = match credentials {
                    #[cfg(feature = "aws-ses")]
                    SesCredentials::Native {
                        access_key_id,
                        secret_access_key,
                        session_token,
                    } => AwsSesDriver::try_native(
                        region,
                        access_key_id,
                        secret_access_key,
                        session_token,
                    )?,
                    SesCredentials::Proxy(token) => AwsSesDriver::try_new(region, token)?,
                };
                if let Some(endpoint) = endpoint {
                    driver = driver.try_with_endpoint(endpoint)?;
                }
                Box::new(driver)
            }
        })
    }

    fn digest(&self) -> [u8; 32] {
        let mut hasher = DigestHasher(Sha256::new());
        self.hash(&mut hasher);
        hasher.0.finalize().into()
    }
}

/// Port 25 only when `MAIL_PORT` is absent or empty; a malformed or
/// out-of-range value fails closed.
#[cfg(feature = "mail-smtp")]
fn smtp_port(setting: Option<String>) -> Result<u16, MailError> {
    match setting {
        Some(port) if !port.trim().is_empty() => port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or_else(|| {
                MailError::ConfigError("MAIL_PORT must be an integer from 1 to 65535".to_string())
            }),
        _ => Ok(25),
    }
}

fn ses_credentials(settings: &MailSettings) -> Result<SesCredentials, MailError> {
    match (
        settings.value("AWS_ACCESS_KEY_ID")?,
        settings.value("AWS_SECRET_ACCESS_KEY")?,
    ) {
        #[cfg(feature = "aws-ses")]
        (Some(access_key_id), Some(secret_access_key)) => Ok(SesCredentials::Native {
            access_key_id,
            secret_access_key,
            session_token: settings.value("AWS_SESSION_TOKEN")?,
        }),
        #[cfg(not(feature = "aws-ses"))]
        (Some(_), Some(_)) => Err(MailError::ConfigError(
            "native AWS SES credentials require the `aws-ses` feature".to_string(),
        )),
        (None, None) => Ok(SesCredentials::Proxy(
            match settings.value("AWS_SES_TOKEN")? {
                Some(token) => token,
                None => settings.value("AWS_SES_BEARER_TOKEN")?.unwrap_or_default(),
            },
        )),
        _ => Err(MailError::ConfigError(
            "native AWS SES requires both AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY".to_string(),
        )),
    }
}

/// The facade's driver for `settings`: the previously built one while the
/// settings are unchanged, otherwise a new one that replaces it. A `memory`
/// driver is built per call, as before, so it never accumulates messages. A
/// poisoned cache only disables reuse.
pub(super) fn configured_driver(settings: &MailSettings) -> Result<Arc<dyn MailDriver>, MailError> {
    let spec = DriverSpec::from_settings(settings)?;
    if matches!(spec, DriverSpec::Memory) {
        return spec.build().map(Arc::from);
    }
    let key = spec.digest();
    if let Ok(cached) = CONFIGURED_DRIVER.lock()
        && let Some((cached_key, driver)) = cached.as_ref()
        && *cached_key == key
    {
        return Ok(Arc::clone(driver));
    }
    let driver: Arc<dyn MailDriver> = Arc::from(spec.build()?);
    if let Ok(mut cached) = CONFIGURED_DRIVER.lock() {
        *cached = Some((key, Arc::clone(&driver)));
    }
    Ok(driver)
}

/// Drops the reused driver, so the next facade call builds a fresh one.
pub(super) fn forget_configured_driver() {
    if let Ok(mut cached) = CONFIGURED_DRIVER.lock() {
        *cached = None;
    }
}

#[cfg(test)]
#[path = "driver_tests.rs"]
mod tests;
