// src/facade.rs — Main Mail facade, queue dispatcher, and runtime driver resolver.

use crate::drivers::*;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use rullst_core::queue::Queue;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::OnceCell;

mod settings;
use settings::MailSettings;
#[cfg(test)]
use settings::default_driver_name;

static MAIL_QUEUE: OnceCell<Queue> = OnceCell::const_new();
static CUSTOM_DRIVER: RwLock<Option<Arc<dyn MailDriver>>> = RwLock::new(None);
#[cfg(test)]
pub(crate) static MAIL_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub(crate) const MAIL_JOB_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QueuedMail {
    pub(crate) schema_version: u8,
    pub(crate) tenant_id: Option<String>,
    pub(crate) message: Message,
}

/// The main Mail facade
pub struct Mail;

impl Mail {
    /// Sets a custom mail driver (e.g. `MemoryDriver` or custom backend) overriding default resolution.
    pub fn set_driver(driver: Box<dyn MailDriver>) {
        if let Ok(mut lock) = CUSTOM_DRIVER.write() {
            *lock = Some(Arc::from(driver));
        }
    }

    /// Clears any custom mail driver, restoring default resolution from env/Rullst.toml.
    pub fn reset_driver() {
        if let Ok(mut lock) = CUSTOM_DRIVER.write() {
            *lock = None;
        }
    }

    /// Initializes the global mail queue.
    /// If configured, `Mail::send` will automatically push emails to this queue.
    pub fn init_queue(queue: Queue) {
        let _ = MAIL_QUEUE.set(queue);
    }

    /// Send a message. If a background queue is initialized, it pushes to the queue automatically.
    /// Otherwise, it sends synchronously.
    ///
    /// A message without a `from` uses the configured default sender (see
    /// [`Mail::default_sender`]); an explicit `from` always wins.
    pub async fn send(message: Message) -> Result<(), MailError> {
        let settings = MailSettings::load().await?;
        let message = settings.apply_default_sender(message)?;
        let message = DeliveryPipeline::prepare(&message)?.into_message();
        if let Some(queue) = MAIL_QUEUE.get() {
            Self::enqueue_prepared(
                queue,
                QueuedMail {
                    schema_version: MAIL_JOB_SCHEMA_VERSION,
                    tenant_id: None,
                    message,
                },
            )
            .await?;
            Ok(())
        } else {
            Self::deliver(&settings, None, message).await
        }
    }

    /// Forces sending the message synchronously, bypassing the background queue.
    pub async fn send_now(message: Message) -> Result<(), MailError> {
        let settings = MailSettings::load().await?;
        let message = settings.apply_default_sender(message)?;
        Self::deliver(&settings, None, message).await
    }

    /// Sends a message for a specific tenant when using a multi-tenant driver or custom resolver.
    pub async fn send_for_tenant(
        tenant_id: impl Into<String>,
        message: Message,
    ) -> Result<(), MailError> {
        let tenant_id = tenant_id.into();
        let settings = MailSettings::load().await?;
        let message = settings.apply_default_sender(message)?;
        let message = DeliveryPipeline::prepare_for_tenant(&tenant_id, &message)?.into_message();
        if let Some(queue) = MAIL_QUEUE.get() {
            Self::enqueue_prepared(
                queue,
                QueuedMail {
                    schema_version: MAIL_JOB_SCHEMA_VERSION,
                    tenant_id: Some(tenant_id),
                    message,
                },
            )
            .await?;
            return Ok(());
        }

        Self::deliver(&settings, Some(&tenant_id), message).await
    }

    /// Forces tenant-aware synchronous delivery, bypassing the background queue.
    pub async fn send_now_for_tenant(
        tenant_id: impl Into<String>,
        message: Message,
    ) -> Result<(), MailError> {
        let tenant_id = tenant_id.into();
        let settings = MailSettings::load().await?;
        let message = settings.apply_default_sender(message)?;
        Self::deliver(&settings, Some(&tenant_id), message).await
    }

    /// Enqueues a message on an explicit queue, preserving its optional `send_at` timestamp.
    pub async fn enqueue(queue: &Queue, message: Message) -> Result<(), MailError> {
        let message = MailSettings::load().await?.apply_default_sender(message)?;
        let message = DeliveryPipeline::prepare(&message)?.into_message();
        Self::enqueue_prepared(
            queue,
            QueuedMail {
                schema_version: MAIL_JOB_SCHEMA_VERSION,
                tenant_id: None,
                message,
            },
        )
        .await
    }

    /// Enqueues a tenant-scoped message on an explicit queue with durable scheduling metadata.
    pub async fn enqueue_for_tenant(
        queue: &Queue,
        tenant_id: impl Into<String>,
        message: Message,
    ) -> Result<(), MailError> {
        let tenant_id = tenant_id.into();
        let message = MailSettings::load().await?.apply_default_sender(message)?;
        let message = DeliveryPipeline::prepare_for_tenant(&tenant_id, &message)?.into_message();
        Self::enqueue_prepared(
            queue,
            QueuedMail {
                schema_version: MAIL_JOB_SCHEMA_VERSION,
                tenant_id: Some(tenant_id),
                message,
            },
        )
        .await
    }

    /// Returns the validated default sender for facade messages without a
    /// `from`: `MAIL_FROM` from the process environment, then `./.env`, else
    /// `from` in the `[mail]` section of `Rullst.toml`. It may be a bare
    /// address or `Name <address>`.
    ///
    /// Every facade send and enqueue validates it and fails with
    /// [`MailError::ConfigError`] when it is invalid; call this at startup to
    /// fail fast instead. Drivers used directly do not read it. (v13)
    pub async fn default_sender() -> Result<Option<String>, MailError> {
        Ok(MailSettings::load()
            .await?
            .default_sender()?
            .map(str::to_string))
    }

    /// Runs the pipeline and dispatches through the custom or configured driver.
    async fn deliver(
        settings: &MailSettings,
        tenant_id: Option<&str>,
        message: Message,
    ) -> Result<(), MailError> {
        let message = match tenant_id {
            Some(tenant_id) => DeliveryPipeline::prepare_for_tenant(tenant_id, &message)?,
            None => DeliveryPipeline::prepare(&message)?,
        }
        .into_message();
        let driver: Arc<dyn MailDriver> = match Self::custom_driver()? {
            Some(driver) => driver,
            None => Arc::from(Self::resolve_driver_from(settings)?),
        };
        match tenant_id {
            Some(tenant_id) => driver.send_for_tenant(tenant_id, &message).await,
            None => driver.send(&message).await,
        }
    }

    fn custom_driver() -> Result<Option<Arc<dyn MailDriver>>, MailError> {
        CUSTOM_DRIVER
            .read()
            .map(|driver| driver.clone())
            .map_err(|_| MailError::DriverError("custom driver lock poisoned".to_string()))
    }

    async fn enqueue_prepared(queue: &Queue, job: QueuedMail) -> Result<(), MailError> {
        let available_at = job
            .message
            .send_at
            .as_ref()
            .map(datetime_to_system_time)
            .transpose()?;
        let payload = serde_json::to_value(job).map_err(|error| {
            MailError::SendError(format!("failed to serialize mail queue job: {error}"))
        })?;
        let result = if let Some(available_at) = available_at {
            queue
                .dispatch_at("rullst_mail_send", payload, available_at)
                .await
        } else {
            queue.dispatch("rullst_mail_send", payload).await
        };
        result.map(|_| ()).map_err(|error| match error {
            // A connection or storage outage may clear, so it is retryable.
            rullst_core::queue::QueueError::Driver(_) => {
                MailError::transport("queue", format!("failed to enqueue mail job: {error}"))
            }
            other => MailError::SendError(format!("failed to enqueue mail job: {other}")),
        })
    }

    #[cfg(test)]
    async fn resolve_driver() -> Result<Box<dyn MailDriver>, MailError> {
        Self::resolve_driver_from(&MailSettings::load().await?)
    }

    #[cfg_attr(mutants, mutants::skip)]
    fn resolve_driver_from(settings: &MailSettings) -> Result<Box<dyn MailDriver>, MailError> {
        let driver_name = settings.driver_name()?;

        match driver_name.as_str() {
            "log" => Ok(Box::new(LogDriver)),
            "memory" => Ok(Box::new(MemoryDriver::new())),
            "smtp" => {
                #[cfg(feature = "mail-smtp")]
                {
                    let host = settings
                        .value("MAIL_HOST")?
                        .unwrap_or_else(|| "127.0.0.1".to_string());
                    let port = settings
                        .value("MAIL_PORT")?
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(25);
                    let username = settings.value("MAIL_USERNAME")?;
                    let password = settings.value("MAIL_PASSWORD")?;

                    Ok(Box::new(SmtpDriver::try_new(
                        host, port, username, password,
                    )?))
                }
                #[cfg(not(feature = "mail-smtp"))]
                {
                    Ok(Box::new(SmtpDriver))
                }
            }
            "resend" => {
                let api_key = settings.value("RESEND_API_KEY")?.unwrap_or_default();
                Ok(Box::new(ResendDriver::try_new(api_key)?))
            }
            "sendpulse" => Ok(Box::new(SendPulseDriver::try_new(
                settings.value("SENDPULSE_API_KEY")?.unwrap_or_default(),
            )?)),
            "mailjet" | "mailjet-sandbox" => {
                let driver = MailjetDriver::try_new(
                    settings.value("MAILJET_API_KEY")?.unwrap_or_default(),
                    settings.value("MAILJET_SECRET_KEY")?.unwrap_or_default(),
                )?;
                Ok(Box::new(if driver_name == "mailjet-sandbox" {
                    driver.with_sandbox()
                } else {
                    driver
                }))
            }
            "mailtrap" => Ok(Box::new(MailtrapDriver::try_new(
                settings.value("MAILTRAP_API_TOKEN")?.unwrap_or_default(),
            )?)),
            "mailtrap-sandbox" => {
                let id = settings
                    .value("MAILTRAP_SANDBOX_ID")?
                    .and_then(|v| v.parse::<u64>().ok())
                    .ok_or_else(|| {
                        MailError::ConfigError(
                            "MAILTRAP_SANDBOX_ID must be a positive integer".into(),
                        )
                    })?;
                Ok(Box::new(MailtrapDriver::sandbox(
                    settings.value("MAILTRAP_API_TOKEN")?.unwrap_or_default(),
                    id,
                )?))
            }
            "sendgrid" => {
                let api_key = settings.value("SENDGRID_API_KEY")?.unwrap_or_default();
                Ok(Box::new(SendGridDriver::try_new(api_key)?))
            }
            "postmark" => {
                let server_token = match settings.value("POSTMARK_SERVER_TOKEN")? {
                    Some(token) => token,
                    None => settings.value("POSTMARK_API_KEY")?.unwrap_or_default(),
                };
                let message_stream = settings.value("POSTMARK_MESSAGE_STREAM")?;
                let mut driver = PostmarkDriver::try_new(server_token)?;
                if let Some(stream) = message_stream {
                    driver = driver.with_message_stream(stream);
                }
                Ok(Box::new(driver))
            }
            "azure-acs" => {
                let endpoint = settings
                    .value("AZURE_COMMUNICATION_EMAIL_ENDPOINT")?
                    .unwrap_or_default();
                if endpoint.is_empty() || endpoint.starts_with("mock_") {
                    Ok(Box::new(AzureCommunicationDriver::new(
                        endpoint,
                        StaticAzureMailCredential::new("mock_azure", 0)?,
                    )?))
                } else {
                    Ok(Box::new(AzureCommunicationDriver::new(
                        endpoint,
                        AzureManagedIdentity::from_environment()?,
                    )?))
                }
            }
            "ses" | "aws_ses" => {
                let region = settings
                    .value("AWS_REGION")?
                    .unwrap_or_else(|| "us-east-1".to_string());
                let endpoint_override = settings.value("AWS_SES_ENDPOINT")?;
                let access_key_id = settings.value("AWS_ACCESS_KEY_ID")?;
                let secret_access_key = settings.value("AWS_SECRET_ACCESS_KEY")?;
                let mut driver = match (access_key_id, secret_access_key) {
                    (Some(access_key_id), Some(secret_access_key)) => {
                        #[cfg(feature = "aws-ses")]
                        {
                            AwsSesDriver::try_native(
                                region,
                                access_key_id,
                                secret_access_key,
                                settings.value("AWS_SESSION_TOKEN")?,
                            )?
                        }
                        #[cfg(not(feature = "aws-ses"))]
                        {
                            let _ = (region, access_key_id, secret_access_key);
                            return Err(MailError::ConfigError(
                                "native AWS SES credentials require the `aws-ses` feature"
                                    .to_string(),
                            ));
                        }
                    }
                    (None, None) => {
                        let auth_token = match settings.value("AWS_SES_TOKEN")? {
                            Some(token) => token,
                            None => settings.value("AWS_SES_BEARER_TOKEN")?.unwrap_or_default(),
                        };
                        AwsSesDriver::try_new(region, auth_token)?
                    }
                    _ => {
                        return Err(MailError::ConfigError(
                            "native AWS SES requires both AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY"
                                .to_string(),
                        ));
                    }
                };
                if let Some(endpoint) = endpoint_override {
                    driver = driver.try_with_endpoint(endpoint)?;
                }
                Ok(Box::new(driver))
            }
            other => Err(MailError::ConfigError(format!(
                "Unknown mail driver: {}",
                other
            ))),
        }
    }
}

fn datetime_to_system_time(
    timestamp: &chrono::DateTime<chrono::Utc>,
) -> Result<SystemTime, MailError> {
    let seconds = u64::try_from(timestamp.timestamp()).map_err(|_| {
        MailError::ValidationError("mail schedule predates the Unix epoch".to_string())
    })?;
    UNIX_EPOCH
        .checked_add(Duration::new(seconds, timestamp.timestamp_subsec_nanos()))
        .ok_or_else(|| MailError::ValidationError("mail schedule exceeds system range".to_string()))
}

#[cfg(test)]
#[path = "facade_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "facade_dotenv_tests.rs"]
mod dotenv_tests;
