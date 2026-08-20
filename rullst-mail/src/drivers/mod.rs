// src/drivers/mod.rs — Driver trait and error definitions.

use crate::message::Message;
use async_trait::async_trait;

pub mod aws_ses;
pub mod failover;
pub mod log;
pub mod memory;
pub mod postmark;
pub mod resend;
pub mod sendgrid;
pub mod smtp;

pub use self::aws_ses::AwsSesDriver;
pub use self::failover::FailoverDriver;
pub use self::log::LogDriver;
pub use self::memory::{MailAssertion, MailTrap, MemoryDriver};
pub use self::postmark::PostmarkDriver;
pub use self::resend::ResendDriver;
pub use self::sendgrid::SendGridDriver;
pub use self::smtp::SmtpDriver;

pub use crate::error::MailError;

#[async_trait]
/// Interface for different email dispatching backends.
pub trait MailDriver: Send + Sync {
    /// Dispatches the given email message.
    async fn send(&self, message: &Message) -> Result<(), MailError>;
}
