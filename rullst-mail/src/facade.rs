// src/facade.rs — Main Mail facade, queue dispatcher, and runtime driver resolver.

use crate::drivers::*;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use rullst_core::queue::Queue;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::OnceCell;

static MAIL_QUEUE: OnceCell<Queue> = OnceCell::const_new();
static CUSTOM_DRIVER: RwLock<Option<Arc<dyn MailDriver>>> = RwLock::new(None);

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
    pub async fn send(message: Message) -> Result<(), MailError> {
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
            Self::send_now(message).await
        }
    }

    /// Forces sending the message synchronously, bypassing the background queue.
    pub async fn send_now(message: Message) -> Result<(), MailError> {
        let message = DeliveryPipeline::prepare(&message)?.into_message();
        let custom_driver = Self::custom_driver()?;
        if let Some(driver) = custom_driver {
            return driver.send(&message).await;
        }
        let driver = Self::resolve_driver().await?;
        driver.send(&message).await
    }

    /// Sends a message for a specific tenant when using a multi-tenant driver or custom resolver.
    pub async fn send_for_tenant(
        tenant_id: impl Into<String>,
        message: Message,
    ) -> Result<(), MailError> {
        let tenant_id = tenant_id.into();
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

        Self::send_now_for_tenant(&tenant_id, message).await
    }

    /// Forces tenant-aware synchronous delivery, bypassing the background queue.
    pub async fn send_now_for_tenant(
        tenant_id: impl Into<String>,
        message: Message,
    ) -> Result<(), MailError> {
        let tenant_id = tenant_id.into();
        let message = DeliveryPipeline::prepare_for_tenant(&tenant_id, &message)?.into_message();
        let custom_driver = Self::custom_driver()?;
        if let Some(driver) = custom_driver {
            driver.send_for_tenant(&tenant_id, &message).await
        } else {
            let driver = Self::resolve_driver().await?;
            driver.send_for_tenant(&tenant_id, &message).await
        }
    }

    /// Enqueues a message on an explicit queue, preserving its optional `send_at` timestamp.
    pub async fn enqueue(queue: &Queue, message: Message) -> Result<(), MailError> {
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
        result
            .map(|_| ())
            .map_err(|error| MailError::SendError(format!("failed to enqueue mail job: {error}")))
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn resolve_driver() -> Result<Box<dyn MailDriver>, MailError> {
        // Resolve the driver either from env or Rullst.toml
        let mut driver_name_opt = std::env::var("MAIL_DRIVER").ok();

        if driver_name_opt.is_none()
            && let Ok(toml_content) = tokio::fs::read_to_string("Rullst.toml").await
        {
            let mut in_mail = false;
            for line in toml_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') {
                    in_mail = trimmed == "[mail]" || trimmed == "[mailer]";
                    continue;
                }
                if in_mail
                    && trimmed.starts_with("driver")
                    && let Some(val) = trimmed.split('=').nth(1)
                {
                    let clean_val = val.split('#').next().unwrap_or(val).trim();
                    driver_name_opt =
                        Some(clean_val.trim_matches('"').trim_matches('\'').to_string());
                }
            }
        }

        let driver_name = driver_name_opt.unwrap_or_else(|| "log".to_string());

        match driver_name.as_str() {
            "log" => Ok(Box::new(LogDriver)),
            "memory" => Ok(Box::new(MemoryDriver::new())),
            "smtp" => {
                #[cfg(feature = "mail-smtp")]
                {
                    let host =
                        std::env::var("MAIL_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                    let port = std::env::var("MAIL_PORT")
                        .ok()
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(25);
                    let username = std::env::var("MAIL_USERNAME").ok();
                    let password = std::env::var("MAIL_PASSWORD").ok();

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
                let api_key = std::env::var("RESEND_API_KEY").unwrap_or_default();
                Ok(Box::new(ResendDriver::try_new(api_key)?))
            }
            "sendgrid" => {
                let api_key = std::env::var("SENDGRID_API_KEY").unwrap_or_default();
                Ok(Box::new(SendGridDriver::try_new(api_key)?))
            }
            "postmark" => {
                let server_token = std::env::var("POSTMARK_SERVER_TOKEN")
                    .or_else(|_| std::env::var("POSTMARK_API_KEY"))
                    .unwrap_or_default();
                let message_stream = std::env::var("POSTMARK_MESSAGE_STREAM").ok();
                let mut driver = PostmarkDriver::try_new(server_token)?;
                if let Some(stream) = message_stream {
                    driver = driver.with_message_stream(stream);
                }
                Ok(Box::new(driver))
            }
            "ses" | "aws_ses" => {
                let region =
                    std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
                let endpoint_override = std::env::var("AWS_SES_ENDPOINT").ok();
                let access_key_id = std::env::var("AWS_ACCESS_KEY_ID").ok();
                let secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
                let mut driver = match (access_key_id, secret_access_key) {
                    (Some(access_key_id), Some(secret_access_key)) => {
                        #[cfg(feature = "aws-ses")]
                        {
                            AwsSesDriver::try_native(
                                region,
                                access_key_id,
                                secret_access_key,
                                std::env::var("AWS_SESSION_TOKEN").ok(),
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
                        let auth_token = std::env::var("AWS_SES_TOKEN")
                            .or_else(|_| std::env::var("AWS_SES_BEARER_TOKEN"))
                            .unwrap_or_default();
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
