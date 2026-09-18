// src/drivers/log.rs — Terminal output and disk file logger mail driver.

use super::traits::MailDriver;
use crate::error::MailError;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;

/// A driver that outputs emails to the terminal and logs to storage/logs/mail.log
pub struct LogDriver;

#[async_trait]
impl MailDriver for LogDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        DeliveryPipeline::require_due("LogDriver", message)?;
        let path_str =
            std::env::var("MAIL_LOG_PATH").unwrap_or_else(|_| "storage/logs/mail.log".to_string());
        let log_path = std::path::PathBuf::from(path_str);
        if let Some(parent) = log_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                MailError::DriverError(format!("Failed to create log directory: {}", e))
            })?;
        }
        // Log delivery metadata only: even intended action tokens must never
        // become plaintext log records. MemoryDriver is the explicit preview.
        let formatted = format!(
            "[MAIL LOGGED] {} attachments={} scheduled={}\n",
            chrono::Utc::now().to_rfc3339(),
            message.attachments.len(),
            message.send_at.is_some(),
        );
        println!(
            "[MAIL LOGGED] {} | Target: {}",
            chrono::Local::now().to_rfc3339(),
            log_path.display()
        );

        let log_path_owned = log_path.clone();
        let formatted_clone = formatted.clone();
        tokio::task::spawn_blocking(move || {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path_owned)
                .map_err(|e| MailError::DriverError(format!("Failed to open log file: {}", e)))?;
            file.write_all(formatted_clone.as_bytes()).map_err(|e| {
                MailError::DriverError(format!("Failed to write to log file: {}", e))
            })?;
            file.flush()
                .map_err(|e| MailError::DriverError(format!("Failed to flush log file: {}", e)))?;
            file.sync_all()
                .map_err(|e| MailError::DriverError(format!("Failed to sync log file: {}", e)))?;
            Ok::<(), MailError>(())
        })
        .await
        .map_err(|e| MailError::DriverError(format!("spawn_blocking error: {}", e)))??;

        Ok(())
    }
}
