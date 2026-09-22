//! Durable outgoing HTTPS webhooks with explicit destination and receiver contracts.
//! Hosts authorize namespaces and receivers deduplicate stable message IDs.
mod config;
mod dispatch;
mod error;
mod outbox;
mod signature;
mod transport;

use crate::{Clock, SystemClock};
pub use config::{WebhookConfig, WebhookDestination};
pub use dispatch::WebhookDispatch;
pub use error::WebhookError;
pub use outbox::{WebhookFailure, WebhookOutbox};
pub use signature::{VerifiedWebhook, WebhookSignature, WebhookSigningKey};
use std::{sync::Arc, time::Duration};

type Result<T> = std::result::Result<T, WebhookError>;
const MAX_BODY: usize = 64 * 1024;
const MAX_TIMESTAMP: i64 = 253_402_300_799_000;
fn now(clock: &impl Clock) -> Result<i64> {
    let value = clock.now_millis().map_err(|_| WebhookError::Clock)?;
    if !(0..=MAX_TIMESTAMP - 7 * 86_400_000).contains(&value) {
        return Err(WebhookError::Clock);
    }
    Ok(value)
}
async fn bounded<T>(operation: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    tokio::time::timeout(Duration::from_secs(10), operation)
        .await
        .map_err(|_| WebhookError::Timeout)?
}
