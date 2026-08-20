// src/error.rs — Core error definitions for rullst-mail.

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
