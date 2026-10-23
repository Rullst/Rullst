//! `db:*` Artisan commands reuse an ORM pool the application opened itself
//! (the `pool_ready` guard of `run_artisan_command`) instead of resolving
//! `DATABASE_URL` again.
//!
//! The ORM pool is process-wide, so the command runs in a child process: this
//! test binary started again with `db:status` as its first argument, which
//! `check_and_run_artisan` intercepts and then exits. The all-feature build
//! that mutation testing measures opens only PostgreSQL pools, so the child
//! connects to a disposable PostgreSQL container. Without Docker the test is
//! skipped, unless `RULLST_REQUIRE_TESTCONTAINERS=true`.
#![cfg(feature = "strict-postgres")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{GenericImage, ImageExt};

/// Set in the child: the URL of the pool it opens before the command runs.
const CHILD_POOL_URL: &str = "RULLST_ARTISAN_TEST_POOL_URL";
const CHILD_TEST: &str = "child_runs_a_database_command_on_its_open_pool";

/// In the child, opens the pool and runs the command named by the process
/// arguments; `check_and_run_artisan` exits the process. Elsewhere it does
/// nothing.
#[tokio::test]
async fn child_runs_a_database_command_on_its_open_pool() {
    let Ok(url) = std::env::var(CHILD_POOL_URL) else {
        return;
    };
    // The image restarts once after initializing; retry within a deadline.
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match rullst_orm::Orm::init(&url).await {
            Ok(()) => break,
            Err(error) if Instant::now() < deadline => {
                eprintln!("PostgreSQL not ready yet: {error}");
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            Err(error) => panic!("PostgreSQL pool did not open: {error}"),
        }
    }
    let _ = rullst_core::artisan::runner::check_and_run_artisan(Vec::new(), Vec::new()).await;
    panic!("check_and_run_artisan returned instead of running `db:status`");
}

#[tokio::test]
async fn database_commands_reuse_the_pool_the_application_opened() {
    let container = match GenericImage::new(
        "postgres",
        "latest@sha256:29342cb52157b098821961d2c14eec3c019071f56a5d559e990cf07cf541ea9b",
    )
    .with_wait_for(WaitFor::message_on_stderr(
        "database system is ready to accept connections",
    ))
    .with_exposed_port(5432.tcp())
    .with_env_var("POSTGRES_PASSWORD", "postgres")
    .start()
    .await
    {
        Ok(container) => container,
        Err(error) => {
            if std::env::var("RULLST_REQUIRE_TESTCONTAINERS").as_deref() == Ok("true") {
                panic!("PostgreSQL testcontainer is required but could not start: {error}");
            }
            eprintln!("skipping the Artisan open-pool contract: {error}");
            return;
        }
    };
    let host = container.get_host().await.expect("PostgreSQL host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("PostgreSQL port");
    // No `.env` or `Rullst.toml`, and a DATABASE_URL nothing listens on: the
    // command must use the pool that is already open, not resolve a URL.
    let project = tempfile::tempdir().expect("empty project directory");
    let mut child = Command::new(std::env::current_exe().expect("test binary"))
        .args(["db:status", CHILD_TEST, "--exact", "--nocapture"])
        .current_dir(project.path())
        .env(
            CHILD_POOL_URL,
            format!("postgres://postgres:postgres@{host}:{port}/postgres"),
        )
        .env(
            "DATABASE_URL",
            "postgres://unused:unused@127.0.0.1:1/unused",
        )
        .env("RULLST_ENV", "development")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("child test process");
    let deadline = Instant::now() + Duration::from_secs(120);
    while child.try_wait().expect("child status").is_none() {
        if Instant::now() >= deadline {
            child.kill().expect("stop the child");
            panic!("the child Artisan command did not finish within 120 seconds");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let output = child.wait_with_output().expect("child output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "db:status failed with an open pool\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    // The status table of the (empty) migration list.
    assert!(
        stdout.contains(&format!("{:<40} | Status", "Migration Name")),
        "{stdout}"
    );
    assert!(!stderr.contains("no database is configured"), "{stderr}");
}
