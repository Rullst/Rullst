//! Shared PostgreSQL state for final pre-dispatch suppression checks.
mod connection;
mod operations;
mod storage;
mod types;

use super::*;
use sqlx::{PgPool, Postgres, Transaction};
pub use types::{PostgresSuppressionConfig, SuppressionKey};

/// Authoritative per-namespace suppression, without stored recipient addresses.
/// The host verifies tenant authority and provider signatures before ingestion.
#[derive(Clone)]
pub struct PostgresSuppressionStore {
    pool: PgPool,
    config: PostgresSuppressionConfig,
    key: SuppressionKey,
}

async fn bounded<T>(
    operation: impl std::future::Future<Output = Result<T, SuppressionError>>,
) -> Result<T, SuppressionError> {
    tokio::time::timeout(std::time::Duration::from_secs(10), operation)
        .await
        .map_err(|_| unavailable("operation deadline"))?
}

fn now() -> Result<i64, SuppressionError> {
    i64::try_from(super::unix_time()?)
        .ok()
        .filter(|value| *value > 0 && *value <= i64::MAX - 300)
        .ok_or(SuppressionError::InvalidConfiguration("server clock"))
}
