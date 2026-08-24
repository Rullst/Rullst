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

    /// Dispatches for a validated tenant context.
    ///
    /// Drivers without tenant-specific credentials safely reuse their normal transport.
    /// Resolvers override this method to select the tenant's isolated driver.
    async fn send_for_tenant(&self, tenant_id: &str, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare_for_tenant(tenant_id, message)?;
        self.send(prepared.message()).await
    }
}
