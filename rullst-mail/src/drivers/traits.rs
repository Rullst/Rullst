// src/drivers/traits.rs — MailDriver trait definition.

use crate::error::MailError;
use crate::message::Message;
use crate::pipeline::DeliveryPipeline;
use async_trait::async_trait;

#[async_trait]
/// Interface for different email dispatching backends.
pub trait MailDriver: Send + Sync {
    /// Dispatches the given email message.
    async fn send(&self, message: &Message) -> Result<(), MailError>;

    /// Sends with a stable outbox identity. The default remains at-least-once;
    /// only providers with native idempotency (currently Resend) override it.
    /// Never assume a Message-ID or this method implies exactly-once delivery.
    async fn send_with_delivery_id(
        &self,
        message: &Message,
        delivery_id: &str,
    ) -> Result<(), MailError> {
        validate_delivery_id(delivery_id)?;
        self.send(message).await
    }

    /// Dispatches for a validated tenant context.
    ///
    /// Drivers without tenant-specific credentials safely reuse their normal transport.
    /// Resolvers override this method to select the tenant's isolated driver.
    async fn send_for_tenant(&self, tenant_id: &str, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare_for_tenant(tenant_id, message)?;
        self.send(prepared.message()).await
    }
}

pub(crate) fn validate_delivery_id(value: &str) -> Result<(), MailError> {
    if !(16..=128).contains(&value.len())
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return Err(MailError::ValidationError(
            "delivery ID must be a bounded opaque identifier".into(),
        ));
    }
    Ok(())
}
