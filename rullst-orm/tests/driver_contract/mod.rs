//! Driver-neutral schema and mutation contracts.
//!
//! `driver_contract_sqlite` runs these checks locally; the live PostgreSQL,
//! MySQL and MariaDB matrices run the same functions against real servers.
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod bulk_delete;
mod timestamps;

/// Runs every driver-neutral contract against the initialized ORM.
pub async fn exercise() {
    timestamps::exercise().await;
    bulk_delete::exercise().await;
}
