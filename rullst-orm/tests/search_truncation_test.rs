//! A shared search index is truncated to `MAX_SEARCH_HITS` before tenant
//! scoping, so a capped engine answer for a scoped model falls back to SQL.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::scout::MAX_SEARCH_HITS;
use rullst_orm::{Error, Orm, SearchEngine, set_search_engine, with_tenant};

#[derive(Clone, Debug, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "truncated_invoices", searchable, tenant_column = "tenant_id")]
struct TruncatedInvoice {
    id: i32,
    tenant_id: String,
    title: String,
}

/// Mimics a provider whose shared index is dominated by tenant B: "overdue"
/// fills the cap with B's IDs, "rare" returns one of A's.
struct SharedIndex;

#[rullst_orm::async_trait]
impl SearchEngine for SharedIndex {
    async fn update(&self, _: &str, _: i32, _: serde_json::Value) -> Result<(), Error> {
        Ok(())
    }

    async fn delete(&self, _: &str, _: i32) -> Result<(), Error> {
        Ok(())
    }

    async fn search(&self, _: &str, query: &str) -> Result<Vec<i32>, Error> {
        let capped = i32::try_from(MAX_SEARCH_HITS).expect("cap fits i32");
        Ok(match query {
            "overdue" => (1..=capped).collect(),
            "rare" => vec![5_002],
            _ => Vec::new(),
        })
    }
}

#[tokio::test]
async fn capped_engine_results_do_not_hide_a_tenants_matches() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize SQLite");
    let pool = Orm::pool().expect("pool");
    rullst_orm::_sqlx::query(
        "CREATE TABLE truncated_invoices (id INTEGER PRIMARY KEY, tenant_id TEXT NOT NULL, title TEXT NOT NULL)",
    )
    .execute(pool)
    .await
    .expect("create invoices");
    rullst_orm::_sqlx::query(
        "INSERT INTO truncated_invoices (id, tenant_id, title) VALUES \
         (1, 'tenant-b', 'overdue b'), (2, 'tenant-b', 'overdue b'), \
         (5001, 'tenant-a', 'overdue a'), (5002, 'tenant-a', 'overdue rare'), (5003, 'tenant-a', 'overdue a')",
    )
    .execute(pool)
    .await
    .expect("seed invoices");
    set_search_engine(SharedIndex).expect("configure offline search");

    let ids = |query: &'static str| {
        with_tenant("tenant-a", async move {
            TruncatedInvoice::search(query)
                .await
                .order_by("id")
                .pluck_i32("id")
                .await
                .expect("scoped search")
        })
    };
    // The engine returned only tenant B's IDs, at the cap: SQL answers instead.
    assert_eq!(ids("overdue").await, [5_001, 5_002, 5_003]);
    // Below the cap the engine's scoped answer is kept.
    assert_eq!(ids("rare").await, [5_002]);
}
