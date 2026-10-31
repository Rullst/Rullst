//! The scenarios. Each one creates its own tables through `Schema`, uses only
//! the ORM API a generated application uses, and records normalized values:
//! no generated id, clock reading or unordered row order reaches a transcript.

mod collation;
mod errors;
mod filters;
mod json;
mod lifecycle;
mod ordering;
mod scores;
mod snapshot;
mod temporal;
mod transactions;
mod values;

use rullst_orm::Error;
use rullst_orm::schema::{Blueprint, Schema};

use crate::transcript::{Scenario, Transcript};

pub async fn run_all(transcript: &mut Transcript) {
    lifecycle::run(&mut transcript.scenario("lifecycle")).await;
    values::run(&mut transcript.scenario("values")).await;
    filters::run(&mut transcript.scenario("filters")).await;
    ordering::run(&mut transcript.scenario("ordering")).await;
    collation::run(&mut transcript.scenario("collation")).await;
    snapshot::run(&mut transcript.scenario("snapshot")).await;
    errors::run(&mut transcript.scenario("errors")).await;
    transactions::run(&mut transcript.scenario("transactions")).await;
    temporal::run(&mut transcript.scenario("temporal")).await;
    json::run(&mut transcript.scenario("json")).await;
}

/// Creates the scenario's table and records the outcome as its `setup` step.
/// On failure the scenario stops: its remaining steps are then reported as
/// missing instead of failing one by one.
pub async fn create_table<F>(scenario: &mut Scenario<'_>, table: &str, columns: F) -> bool
where
    F: FnOnce(&mut Blueprint),
{
    let created = Schema::create(table, columns).await;
    let ok = created.is_ok();
    scenario.outcome(&format!("setup.{table}"), created);
    ok
}

/// The names of `rows`, sorted, for queries that specify no order.
pub fn sorted<T>(rows: Result<Vec<T>, Error>, name: fn(&T) -> &str) -> Result<Vec<String>, Error> {
    rows.map(|rows| {
        let mut names: Vec<String> = rows.iter().map(|row| name(row).to_string()).collect();
        names.sort();
        names
    })
}

/// The names of `rows` in the order the query returned them.
pub fn in_order<T>(
    rows: Result<Vec<T>, Error>,
    name: fn(&T) -> &str,
) -> Result<Vec<String>, Error> {
    rows.map(|rows| rows.iter().map(|row| name(row).to_string()).collect())
}
