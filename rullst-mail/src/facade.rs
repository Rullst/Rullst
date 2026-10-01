// src/facade.rs — Main Mail facade, queue dispatcher, and runtime driver resolver.

use crate::drivers::*;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use rullst_core::queue::Queue;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::OnceCell;

mod driver;
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
    ///
    /// The driver built from those settings is reused across messages while
    /// they stay the same; this also drops it, so the next send builds a new one.
    pub fn reset_driver() {
        if let Ok(mut lock) = CUSTOM_DRIVER.write() {
            *lock = None;
        }
        driver::forget_configured_driver();
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
            None => driver::configured_driver(settings)?,
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

    /// Builds a fresh driver from `settings`, bypassing the reused one.
    #[cfg(test)]
    fn resolve_driver_from(settings: &MailSettings) -> Result<Box<dyn MailDriver>, MailError> {
        driver::DriverSpec::from_settings(settings)?.build()
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
