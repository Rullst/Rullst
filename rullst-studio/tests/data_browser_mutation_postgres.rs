// Runs under `strict-postgres` and under the default `sqlx::Any` build, whose
// `QueryBuilder` writes `?` markers that Studio must renumber for PostgreSQL.
#![cfg(all(
    debug_assertions,
    not(feature = "strict-mysql"),
    not(feature = "strict-sqlite")
))]

mod support;

use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn data_browser_mutates_postgres_by_primary_key() {
    // PostgreSQL 12+ reports information-schema identifiers as `name`, which
    // the Any driver cannot decode; the module's default image is version 11.
    let container = match Postgres::default().with_tag("15").start().await {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("PostgreSQL", error);
            return;
        }
    };
    let host = container.get_host().await.expect("PostgreSQL host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("PostgreSQL port");
    support::exercise_mutations(
        &format!("postgres://postgres:postgres@{host}:{port}/postgres"),
        "postgres",
        "studio_mutation_postgres",
    )
    .await;
}
