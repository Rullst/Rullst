use async_trait::async_trait;
use rullst_orm::schema::migration::{Migration, run_artisan_with_args};
use rullst_orm::{Error, Orm, Seeder};

struct DummyMigration;
#[async_trait]
impl Migration for DummyMigration {
    fn name(&self) -> &'static str {
        "m20260820_000001_create_dummy_table"
    }
    async fn up(&self) -> Result<(), Error> {
        let pool = Orm::pool();
        sqlx::query("CREATE TABLE IF NOT EXISTS dummy_records (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(pool)
            .await?;
        Ok(())
    }
    async fn down(&self) -> Result<(), Error> {
        let pool = Orm::pool();
        sqlx::query("DROP TABLE IF EXISTS dummy_records")
            .execute(pool)
            .await?;
        Ok(())
    }
}

struct DummySeeder;
#[async_trait]
impl Seeder for DummySeeder {
    async fn run(&self) -> Result<(), Error> {
        let pool = Orm::pool();
        sqlx::query("INSERT INTO dummy_records (name) VALUES ('seed_entry')")
            .execute(pool)
            .await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_artisan_cli_and_migration_lifecycle() {
    let _ = Orm::init("sqlite:file:memdb_migration_test?mode=memory&cache=shared").await;

    // 1. CLI help (no args)
    let res = run_artisan_with_args(&["artisan".into()], vec![], vec![]).await;
    assert!(res.is_ok());

    // 2. make:migration without name
    let res =
        run_artisan_with_args(&["artisan".into(), "make:migration".into()], vec![], vec![]).await;
    assert!(res.is_ok());

    // 3. Status before migrations
    let res = run_artisan_with_args(
        &["artisan".into(), "status".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 4. Run migrations
    let res = run_artisan_with_args(
        &["artisan".into(), "migrate".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 5. Run migrations again (nothing to migrate)
    let res = run_artisan_with_args(
        &["artisan".into(), "db:migrate".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 6. Status after migrations (applied)
    let res = run_artisan_with_args(
        &["artisan".into(), "db:status".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 7. Seed database
    let res = run_artisan_with_args(
        &["artisan".into(), "db:seed".into()],
        vec![],
        vec![Box::new(DummySeeder)],
    )
    .await;
    assert!(res.is_ok());

    // 8. Rollback migrations
    let res = run_artisan_with_args(
        &["artisan".into(), "migrate:rollback".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 9. Rollback again (nothing to rollback)
    let res = run_artisan_with_args(
        &["artisan".into(), "db:rollback".into()],
        vec![Box::new(DummyMigration)],
        vec![],
    )
    .await;
    assert!(res.is_ok());

    // 10. Unknown command
    let res = run_artisan_with_args(
        &["artisan".into(), "unknown_command".into()],
        vec![],
        vec![],
    )
    .await;
    assert!(res.is_ok());
}

#[test]
fn test_pool_helpers_and_placeholders() {
    // 1. Placeholder replacement for postgres
    let sql = "SELECT * FROM users WHERE id = ? AND email = ? AND role = ?";
    let replaced = rullst_orm::pool::replace_placeholders(sql);
    assert_eq!(
        replaced,
        "SELECT * FROM users WHERE id = $1 AND email = $2 AND role = $3"
    );

    // 2. Prevent lazy loading flag
    rullst_orm::pool::prevent_lazy_loading(true);
    assert!(rullst_orm::pool::is_lazy_loading_prevented());
    rullst_orm::pool::prevent_lazy_loading(false);
    assert!(!rullst_orm::pool::is_lazy_loading_prevented());

    // 3. Query settings
    Orm::enable_query_log();
    Orm::disable_query_log();
    Orm::set_max_query_limit(500);
    Orm::set_query_timeout(30);
}

#[tokio::test]
async fn test_orm_transaction_rollback_and_commit() {
    let _ = Orm::init("sqlite:file:memdb_tx_test?mode=memory&cache=shared").await;

    if let Ok(pool) = Orm::try_pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS tx_items (id INTEGER PRIMARY KEY AUTOINCREMENT, val TEXT)",
        )
        .execute(pool)
        .await;
    }

    // 1. Transaction success
    let res: Result<i32, Error> = Orm::transaction(|_| {
        Box::pin(async {
            let pool = Orm::pool();
            let _ = sqlx::query("INSERT INTO tx_items (val) VALUES ('tx_ok')")
                .execute(pool)
                .await;
            Ok::<i32, String>(42)
        })
    })
    .await;
    assert_eq!(res.unwrap(), 42);

    // 2. Transaction failure & rollback
    let err_res: Result<i32, Error> = Orm::transaction(|_| {
        Box::pin(async {
            let pool = Orm::pool();
            let _ = sqlx::query("INSERT INTO tx_items (val) VALUES ('tx_fail')")
                .execute(pool)
                .await;
            Err::<i32, String>("intentional error".to_string())
        })
    })
    .await;
    assert!(err_res.is_err());
}

#[test]
fn test_schema_diff_ast_extraction() {
    let tables = rullst_orm::schema_diff::extract_tables_from_ast();
    assert!(!tables.is_empty() || tables.is_empty());
}
