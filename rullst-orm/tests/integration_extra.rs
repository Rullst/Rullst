// Extra database integration tests for rullst-orm covering Artisan commands,
// seeders, query logs, custom pool options, and audit log DB integration.

#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::schema::{Blueprint, Migration, Schema, run_artisan_with_args};
use rullst_orm::{Error, Orm, Seeder};
use std::fs;

// Define a dummy migration
struct DummyMigration;

#[async_trait::async_trait]
impl Migration for DummyMigration {
    fn name(&self) -> &'static str {
        "m20260624000000_dummy"
    }

    async fn up(&self) -> Result<(), Error> {
        Schema::create("extra_dummy_table", |t: &mut Blueprint| {
            t.id();
            t.string("title").not_null();
        })
        .await
    }

    async fn down(&self) -> Result<(), Error> {
        Schema::drop_if_exists("extra_dummy_table").await
    }
}

// Define a dummy seeder
struct DummySeeder;

#[async_trait::async_trait]
impl Seeder for DummySeeder {
    async fn run(&self) -> Result<(), Error> {
        // Just execute a query
        let pool = Orm::pool();
        sqlx::query("SELECT 1").execute(pool).await?;
        Ok(())
    }
}

const DB_FILE: &str = "it_extra.db";

#[tokio::test]
async fn integration_extra_suite() {
    let _ = fs::remove_file(DB_FILE);

    // 1. Test Orm::init_with_options
    Orm::init_with_options(&format!("sqlite:{}?mode=rwc", DB_FILE), 10, 5)
        .await
        .expect("Orm::init_with_options should succeed");

    // 2. Test Orm::seed
    Orm::seed(vec![Box::new(DummySeeder)])
        .await
        .expect("Orm::seed should succeed");

    // 2b. Test begin_transaction
    let tx = Orm::begin_transaction()
        .await
        .expect("begin_transaction should succeed");
    tx.rollback().await.expect("rollback should succeed");

    // 3. Test validate_dsn via direct init call error path (already initialized)
    let init_err = Orm::init(&format!("sqlite:{}?mode=rwc", DB_FILE)).await;
    assert!(
        init_err.is_err(),
        "Expected error when initializing already initialized Orm"
    );

    let init_options_err =
        Orm::init_with_options(&format!("sqlite:{}?mode=rwc", DB_FILE), 5, 5).await;
    assert!(
        init_options_err.is_err(),
        "Expected error when initializing already initialized Orm"
    );

    let init_replicas_err =
        Orm::init_with_replicas(&format!("sqlite:{}?mode=rwc", DB_FILE), vec![]).await;
    assert!(
        init_replicas_err.is_err(),
        "Expected error when initializing already initialized Orm"
    );

    // 4. Test run_artisan_with_args commands
    // 4a. make:migration command with no args
    run_artisan_with_args(
        &["artisan".to_string(), "make:migration".to_string()],
        vec![Box::new(DummyMigration)],
        vec![Box::new(DummySeeder)],
    )
    .await
    .expect("make:migration with no args should succeed with printing help/error");

    // 4b. make:migration command with name
    let m_name = "test_table";
    run_artisan_with_args(
        &[
            "artisan".to_string(),
            "make:migration".to_string(),
            m_name.to_string(),
        ],
        vec![Box::new(DummyMigration)],
        vec![Box::new(DummySeeder)],
    )
    .await
    .expect("make:migration with name should succeed");

    // Clean up created migration files
    let _ = fs::remove_dir_all("src/migrations");
    let _ = fs::remove_dir_all("rullst-orm/src/migrations");

    // 4c. migrate command
    run_artisan_with_args(
        &["artisan".to_string(), "migrate".to_string()],
        vec![Box::new(DummyMigration)],
        vec![Box::new(DummySeeder)],
    )
    .await
    .expect("artisan migrate should succeed");

    // 4d. status command
    run_artisan_with_args(
        &["artisan".to_string(), "status".to_string()],
        vec![Box::new(DummyMigration)],
        vec![Box::new(DummySeeder)],
    )
    .await
    .expect("artisan status should succeed");

    // 4e. db:seed command
    run_artisan_with_args(
        &["artisan".to_string(), "db:seed".to_string()],
        vec![Box::new(DummyMigration)],
        vec![Box::new(DummySeeder)],
    )
    .await
    .expect("artisan db:seed should succeed");

    // 4f. rollback command
    run_artisan_with_args(
        &["artisan".to_string(), "migrate:rollback".to_string()],
        vec![Box::new(DummyMigration)],
        vec![Box::new(DummySeeder)],
    )
    .await
    .expect("artisan migrate:rollback should succeed");

    // 4g. Unknown command
    run_artisan_with_args(
        &["artisan".to_string(), "unknown_command".to_string()],
        vec![Box::new(DummyMigration)],
        vec![Box::new(DummySeeder)],
    )
    .await
    .expect("unknown artisan command should succeed with unknown message");

    // 4h. Help (no args)
    run_artisan_with_args(
        &["artisan".to_string()],
        vec![Box::new(DummyMigration)],
        vec![Box::new(DummySeeder)],
    )
    .await
    .expect("artisan help should succeed");

    // 5. Test Audit table creation and writing
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create_audit_table should succeed");

    rullst_orm::audit::log_audit(
        "User",
        1,
        "create",
        None,
        Some(r#"{"name":"Alice"}"#.to_string()),
    )
    .await
    .expect("log_audit should succeed");

    // log_audit_diff with normal values
    rullst_orm::audit::log_audit_diff(
        "User",
        1,
        "update",
        r#"{"name":"Alice"}"#,
        r#"{"name":"Bob"}"#,
    )
    .await
    .expect("log_audit_diff should succeed");

    // log_audit_diff with large values (> 5MB)
    let large_json = format!(r#"{{"data":"{}"}}"#, "A".repeat(5 * 1024 * 1024 + 1));
    rullst_orm::audit::log_audit_diff("User", 1, "update", &large_json, r#"{"name":"Bob"}"#)
        .await
        .expect("log_audit_diff with large payload should succeed");

    // Clean up
    let _ = fs::remove_file(DB_FILE);
}
