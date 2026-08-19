// src/facade.rs — Main Mail facade, queue dispatcher, and runtime driver resolver.

use crate::drivers::*;
use crate::message::Message;
use rullst_core::queue::Queue;
use std::sync::{Arc, RwLock};
use tokio::sync::OnceCell;

static MAIL_QUEUE: OnceCell<Queue> = OnceCell::const_new();
static CUSTOM_DRIVER: RwLock<Option<Arc<dyn MailDriver>>> = RwLock::new(None);

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
        if let Some(queue) = MAIL_QUEUE.get() {
            let payload = serde_json::to_value(&message).map_err(|e| {
                MailError::SendError(format!("Failed to serialize message for queue: {}", e))
            })?;
            queue
                .dispatch("rullst_mail_send", payload)
                .await
                .map_err(|e| MailError::SendError(format!("Failed to enqueue mail job: {}", e)))?;
            Ok(())
        } else {
            Self::send_now(message).await
        }
    }

    /// Forces sending the message synchronously, bypassing the background queue.
    pub async fn send_now(message: Message) -> Result<(), MailError> {
        let custom_driver = CUSTOM_DRIVER.read().ok().and_then(|l| l.clone());
        if let Some(driver) = custom_driver {
            return driver.send(&message).await;
        }
        let driver = Self::resolve_driver().await?;
        driver.send(&message).await
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

                    Ok(Box::new(SmtpDriver {
                        host,
                        port,
                        username,
                        password,
                    }))
                }
                #[cfg(not(feature = "mail-smtp"))]
                {
                    Ok(Box::new(SmtpDriver))
                }
            }
            "resend" => {
                let api_key = std::env::var("RESEND_API_KEY").map_err(|_| {
                    MailError::ConfigError(
                        "RESEND_API_KEY environment variable is not set".to_string(),
                    )
                })?;
                Ok(Box::new(ResendDriver { api_key }))
            }
            "sendgrid" => {
                let api_key = std::env::var("SENDGRID_API_KEY").map_err(|_| {
                    MailError::ConfigError(
                        "SENDGRID_API_KEY environment variable is not set".to_string(),
                    )
                })?;
                Ok(Box::new(SendGridDriver { api_key }))
            }
            other => Err(MailError::ConfigError(format!(
                "Unknown mail driver: {}",
                other
            ))),
        }
    }
}
