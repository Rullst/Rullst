// src/drivers/traits.rs — MailDriver trait definition.

use crate::error::MailError;
use crate::message::Message;
use async_trait::async_trait;

#[async_trait]
/// Interface for different email dispatching backends.
pub trait MailDriver: Send + Sync {
    /// Dispatches the given email message.
    async fn send(&self, message: &Message) -> Result<(), MailError>;
}
