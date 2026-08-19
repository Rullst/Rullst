// src/drivers/mod.rs — Driver trait and error definitions.

use crate::message::Message;
use async_trait::async_trait;

pub mod log;
pub mod memory;
pub mod resend;
pub mod sendgrid;
pub mod smtp;

pub use self::log::LogDriver;
pub use self::memory::{MailAssertion, MailTrap, MemoryDriver};
pub use self::resend::ResendDriver;
pub use self::sendgrid::SendGridDriver;
pub use self::smtp::SmtpDriver;

#[derive(Debug)]
/// Errors that can occur during mail operations.
pub enum MailError {
    /// Configuration errors (e.g. missing API keys).
    ConfigError(String),
    /// Errors occurred while sending the message.
    SendError(String),
    /// Errors related to the driver backend itself.
    DriverError(String),
}

impl std::fmt::Display for MailError {
    #[cfg_attr(mutants, mutants::skip)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailError::ConfigError(err) => write!(f, "Configuration error: {}", err),
            MailError::SendError(err) => write!(f, "Send error: {}", err),
            MailError::DriverError(err) => write!(f, "Driver error: {}", err),
        }
    }
}

impl std::error::Error for MailError {}

#[async_trait]
/// Interface for different email dispatching backends.
pub trait MailDriver: Send + Sync {
    /// Dispatches the given email message.
    async fn send(&self, message: &Message) -> Result<(), MailError>;
}
