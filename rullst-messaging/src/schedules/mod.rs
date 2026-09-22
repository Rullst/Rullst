//! Durable UTC recurring publications, with fenced PostgreSQL occurrence leases.
//! Hosts own tenant authorization and consumer-side effect deduplication.
mod config;
mod connection;
mod crypto;
mod delivery;
mod error;
mod inspection;
mod management;
mod model;
mod relay;
mod schema;
mod storage;
mod tick;

pub use config::{MissedRunPolicy, RecurringConfig, RecurringDefinition, ScheduledMessage};
pub use error::{RecurringError, RecurringRelayError};
pub use model::{
    OccurrenceLease, OccurrenceMetadata, OccurrenceState, RecurringMetadata, RecurringRelayReceipt,
};

use crate::{Clock, MessagingKeyring, SystemClock};
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;

#[derive(Clone)]
pub struct PostgresRecurringStore<C = SystemClock> {
    pool: PgPool,
    config: RecurringConfig,
    keys: Arc<MessagingKeyring>,
    clock: C,
}

type Result<T> = std::result::Result<T, RecurringError>;
const MAX_CONTENT_BYTES: usize = 128 * 1024;
const MAX_PAYLOAD_BYTES: usize = 16 * 1024;
const MAX_ATTEMPTS: i64 = 10;
const MAX_TIMESTAMP: i64 = 253_402_300_799_000;

async fn bounded<T>(operation: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    tokio::time::timeout(std::time::Duration::from_secs(10), operation)
        .await
        .map_err(|_| RecurringError::Storage)?
}
fn current(clock: &impl Clock) -> Result<i64> {
    clock
        .now_millis()
        .map_err(|_| RecurringError::Clock)
        .and_then(|value| {
            if (0..=MAX_TIMESTAMP - 8 * 86400 * 1000).contains(&value) {
                Ok(value)
            } else {
                Err(RecurringError::Clock)
            }
        })
}
