// src/drivers/log.rs — Terminal output and disk file logger mail driver.

use super::traits::MailDriver;
use crate::error::MailError;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;

/// A driver that outputs emails to the terminal and logs to storage/logs/mail.log
///
/// `MAIL_LOG_PATH` overrides the file. Like the `Mail` facade settings, it is
/// read from the process environment first and then from `./.env`.
pub struct LogDriver;

/// The current time at the system time zone's offset, as `+hh:mm`.
fn local_now() -> String {
    let now = jiff::Timestamp::now();
    let offset = jiff::tz::TimeZone::system().to_offset(now);
    rullst_core::rfc3339::format_with_offset(now, offset.seconds())
}

#[async_trait]
impl MailDriver for LogDriver {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare(message)?;
        let message = prepared.message();
        DeliveryPipeline::require_due("LogDriver", message)?;
        let path_str = rullst_core::server::read_project_setting("MAIL_LOG_PATH")
            .await
            .map_err(|error| {
                MailError::ConfigError(format!("mail settings could not be read: {error}"))
            })?
            .unwrap_or_else(|| "storage/logs/mail.log".to_string());
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
            rullst_core::rfc3339::format_with_offset(jiff::Timestamp::now(), 0),
            message.attachments.len(),
            message.send_at.is_some(),
        );
        println!(
            "[MAIL LOGGED] {} | Target: {}",
            local_now(),
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

#[cfg(test)]
mod tests {
    #[test]
    fn the_console_time_is_rfc3339_with_an_offset() {
        let text = super::local_now();
        assert!(
            rullst_core::rfc3339::parse_unix_seconds(&text).is_some(),
            "{text}"
        );
        assert!(!text.ends_with('Z'), "{text}");
    }
}
