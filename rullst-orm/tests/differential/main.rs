//! Differential database tests: the same ORM operations, through the public
//! `#[derive(Orm)]`, query-builder, `Schema` and `Orm::transaction` API that a
//! generated application uses, run on SQLite, PostgreSQL, MySQL and MariaDB.
//! Each backend writes a normalized transcript of what it observed (values,
//! row counts, ordering where the query specifies one, error kinds and
//! transaction outcomes), and every transcript is compared with SQLite's. A
//! difference passes only when `expected.rs` documents it, with its reason
//! and the exact value the backend must produce.
//!
//! The ORM pool is process-wide and each strict driver feature selects one
//! pool type, so one build runs the backends of its driver, each in a child
//! process (this binary again, running only `worker`):
//!
//! - `strict-sqlite` runs SQLite twice, on fresh files in the temporary
//!   directory, and requires identical transcripts (the harness records no
//!   time- or id-dependent value);
//! - `strict-postgres` runs PostgreSQL when `RULLST_DIFF_POSTGRES_URL` is set;
//! - `strict-mysql` runs MySQL and MariaDB when `RULLST_DIFF_MYSQL_URL` and
//!   `RULLST_DIFF_MARIADB_URL` are set.
//!
//! A server URL needs an account that may create databases: each run creates
//! a fresh `rullst_diff_*` database and drops it after a successful run.
//! Transcripts are written to `RULLST_DIFF_DIR` (default:
//! `target/tmp/rullst-differential`), so run the `strict-sqlite` build first:
//!
//! ```text
//! cargo test -p rullst-orm --no-default-features --features strict-sqlite --test differential
//! RULLST_DIFF_POSTGRES_URL=postgres://… cargo test -p rullst-orm --no-default-features --features strict-postgres --test differential
//! RULLST_DIFF_MYSQL_URL=mysql://… RULLST_DIFF_MARIADB_URL=mysql://… cargo test -p rullst-orm --no-default-features --features strict-mysql --test differential
//! ```
//!
//! `RULLST_DIFF_REQUIRE_ALL=1` (set by CI) turns a missing URL or a missing
//! SQLite transcript into a failure instead of a skip.
#![cfg(any(
    all(
        feature = "strict-sqlite",
        not(feature = "strict-postgres"),
        not(feature = "strict-mysql")
    ),
    all(
        feature = "strict-postgres",
        not(feature = "strict-mysql"),
        not(feature = "strict-sqlite")
    ),
    all(
        feature = "strict-mysql",
        not(feature = "strict-postgres"),
        not(feature = "strict-sqlite")
    )
))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod backend;
mod compare;
mod driver;
mod expected;
mod scenarios;
mod transcript;

/// In a child started by `backends_match_sqlite_or_a_documented_difference`,
/// runs every scenario on one backend and writes its transcript. Elsewhere
/// it does nothing.
#[tokio::test]
async fn worker() {
    if let Some(job) = driver::WorkerJob::from_env() {
        backend::run_worker(job.backend, &job.output).await;
    }
}

#[test]
fn backends_match_sqlite_or_a_documented_difference() {
    if driver::WorkerJob::from_env().is_none() {
        driver::run();
    }
}

#[test]
fn documented_differences_are_well_formed() {
    expected::validate();
}
