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
        let path_str =
            std::env::var("MAIL_LOG_PATH").unwrap_or_else(|_| "storage/logs/mail.log".to_string());
        let log_path = std::path::PathBuf::from(path_str);
        if let Some(parent) = log_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                MailError::DriverError(format!("Failed to create log directory: {}", e))
            })?;
        }
        let unsub_info = if let Some(unsub) = message.list_unsubscribe_header() {
            format!("List-Unsubscribe: {}\n", unsub)
        } else {
            String::new()
        };
        let formatted = format!(
            "========================================\n[MAIL SENT] {}\nTo: {}\nFrom: {}\nSubject: {}\n{}----------------------------------------\n[TEXT BODY]\n{}\n----------------------------------------\n[HTML BODY]\n{}\n========================================\n\n",
            chrono::Local::now().to_rfc3339(),
            message.to,
            message.from.as_deref().unwrap_or("noreply@rullst.dev"),
            message.subject,
            unsub_info,
            message.body_text.as_deref().unwrap_or(""),
            message.body_html.as_deref().unwrap_or("")
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
