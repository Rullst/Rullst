//! Driver-neutral schema and mutation contracts.
//!
//! `driver_contract_sqlite` runs these checks locally; the live PostgreSQL,
//! MySQL and MariaDB matrices run the same functions against real servers.
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod aggregate_counts;
mod bulk_delete;
mod column_types;
mod id_only_rows;
mod missing_rows;
mod nested_transaction;
mod offset_paging;
mod search_relevance;
mod soft_delete_lifecycle;
mod timestamps;

/// Runs every driver-neutral contract against the initialized ORM.
pub async fn exercise() {
    timestamps::exercise().await;
    bulk_delete::exercise().await;
    soft_delete_lifecycle::exercise().await;
    nested_transaction::exercise().await;
    column_types::exercise().await;
    aggregate_counts::exercise().await;
    offset_paging::exercise().await;
    missing_rows::exercise().await;
    id_only_rows::exercise().await;
    search_relevance::exercise().await;
}
