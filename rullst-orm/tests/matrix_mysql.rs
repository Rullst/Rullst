#![cfg(not(any(feature = "strict-sqlite", feature = "strict-postgres")))]

mod partial_update_contract;
mod support;

// Driver-neutral contracts, also run on SQLite by `driver_contract_sqlite`.
mod driver_contract;

use rullst_orm::Orm;
use rullst_orm::schema::{Blueprint, Schema};
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mysql::Mysql;

#[derive(Debug, Clone, Orm)]
#[orm(table = "mysql_users", auditable)]
struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(rullst_orm::Enum, Debug, Clone, Copy, PartialEq, Eq)]
#[rullst_enum(type_name = "mysql_account_status", rename_all = "snake_case")]
enum AccountStatus {
    AwaitingReview,
    Active,
}

#[tokio::test]
async fn test_matrix_mysql_crud() {
    // 1. Inicia o container do MySQL
    // Pin to MySQL 8.0: the --default-authentication-plugin flag was removed in 8.4+
    let container = match Mysql::default()
        .with_tag("8.0")
        .with_env_var("MYSQL_ROOT_PASSWORD", "root")
        .with_env_var("MYSQL_DATABASE", "testdb")
        .start()
        .await
    {
        Ok(c) => c,
        Err(e) => {
            support::handle_container_start_error("MySQL", e);
            return;
        }
    };

    let host_ip = container.get_host().await.expect("Failed to get host IP");
    let host_port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("Failed to get port");

    let connection_string = format!("mysql://root:root@{}:{}/testdb", host_ip, host_port);

    // 2. Inicializa o ORM com o MySQL real
    Orm::init(&connection_string)
        .await
        .expect("Orm::init should succeed with MySQL");

    // 3. Cria a tabela (Schema Builder deve funcionar no MySQL)
    Schema::create("mysql_users", |t: &mut Blueprint| {
        t.id();
        t.string("name").not_null();
        t.string("email").not_null();
    })
    .await
    .expect("create mysql_users");
    rullst_orm::audit::create_audit_table()
        .await
        .expect("create MySQL audit table");
    let audit_context =
        rullst_orm::audit::AuditContext::system("mysql-matrix").expect("valid audit context");

    // 4. Executa um CRUD básico
    let mut user = User {
        id: 0,
        name: "Alice MySQL".into(),
        email: "alice@mysql.com".into(),
    };

    // INSERT
    rullst_orm::audit::with_audit_context(audit_context.clone(), user.save())
        .await
        .expect("save new user to mysql");
    assert!(
        user.id > 0,
        "id must be assigned after save (LAST_INSERT_ID)"
    );

    // SELECT
    let found = User::find(user.id)
        .await
        .expect("find query executed")
        .expect("user exists");
    assert_eq!(found.name, "Alice MySQL");

    // UPDATE
    user.name = "Alice MySQL Updated".into();
    rullst_orm::audit::with_audit_context(audit_context.clone(), user.save())
        .await
        .expect("update user in mysql");

    let updated = User::find(user.id).await.unwrap().unwrap();
    assert_eq!(updated.name, "Alice MySQL Updated");
    let audit_id: (i32,) = sqlx::query_as(
        "SELECT id FROM rullst_audits WHERE model_type = ? AND model_id = ? AND event = 'updated' ORDER BY id DESC LIMIT 1",
    )
    .bind("mysql_users")
    .bind(user.id)
    .fetch_one(Orm::pool().expect("MySQL pool"))
    .await
    .expect("read MySQL audit revision");
    user = rullst_orm::audit::with_audit_context(
        audit_context.clone(),
        user.restore_revision(audit_id.0, "matrix rollback"),
    )
    .await
    .expect("restore MySQL revision");
    assert_eq!(user.name, "Alice MySQL");

    // DELETE
    rullst_orm::audit::with_audit_context(audit_context, user.delete())
        .await
        .expect("delete executed");

    let not_found = User::find(user.id).await.unwrap();
    assert!(not_found.is_none());

    support::exercise_outbox().await;
    support::exercise_large_audit_payload().await;
    support::migrations::exercise_foreign_migrations_table().await;
    support::migrations::exercise_concurrent_migration_runners().await;
    support::migrations::exercise_transactional_migrations().await;
    support::migrations::exercise_pool_access_in_transactional_migrations().await;
    partial_update_contract::exercise().await;
    exercise_native_enum().await;
    driver_contract::exercise().await;
    #[cfg(feature = "strict-mysql")]
    support::json_columns::exercise_sqlx_json("JSON").await;
    support::migrations::exercise_runner_waits_past_one_lock_attempt().await;
    exercise_legacy_outbox_key_collation().await;
    exercise_audit_v1_upgrade().await;
}

/// Counts the columns of `table` that match `condition` in this database.
async fn count_columns(table: &str, condition: &str) -> i64 {
    let sql = format!(
        "SELECT COUNT(*) FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = '{table}' AND {condition}"
    );
    let (count,): (i64,) = sqlx::query_as(sqlx::AssertSqlSafe(sql.as_str()))
        .fetch_one(Orm::pool().expect("MySQL pool"))
        .await
        .expect("count the matching columns");
    count
}

/// An outbox created before its keys compared case-sensitively is converted
/// in place by `Outbox::install`.
async fn exercise_legacy_outbox_key_collation() {
    let pool = Orm::pool().expect("MySQL pool");
    sqlx::query("DROP TABLE IF EXISTS rullst_outbox")
        .execute(pool)
        .await
        .expect("drop the outbox");
    sqlx::query(
        "CREATE TABLE rullst_outbox (id BIGINT AUTO_INCREMENT PRIMARY KEY, \
         stream VARCHAR(128) NOT NULL, event_key VARCHAR(128) NOT NULL, \
         event_kind VARCHAR(128) NOT NULL, payload_json LONGTEXT NOT NULL, \
         status VARCHAR(16) NOT NULL, attempts INT NOT NULL, claimed_by VARCHAR(128) NOT NULL, \
         claim_key VARCHAR(128) NOT NULL, claim_expires_at_epoch BIGINT NOT NULL, \
         last_error VARCHAR(512) NOT NULL, available_at_epoch BIGINT NOT NULL, \
         created_at_epoch BIGINT NOT NULL, delivered_at_epoch BIGINT NULL, \
         insert_token VARCHAR(32) NOT NULL, \
         UNIQUE KEY rullst_outbox_stream_event_unique (stream, event_key), \
         INDEX rullst_outbox_delivery_idx (stream, status, available_at_epoch, claim_expires_at_epoch, id)) \
         CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci",
    )
    .execute(pool)
    .await
    .expect("create an outbox with case-insensitive keys");
    let binary_keys = "COLUMN_NAME IN ('stream', 'event_key', 'claim_key') \
                       AND COLLATION_NAME = 'ascii_bin'";
    assert_eq!(count_columns("rullst_outbox", binary_keys).await, 0);

    rullst_orm::Outbox::install()
        .await
        .expect("install converts the legacy outbox");
    assert_eq!(count_columns("rullst_outbox", binary_keys).await, 3);
}

/// A v1 audit table gains its v2 columns, with a `LONGTEXT` restore patch.
async fn exercise_audit_v1_upgrade() {
    let pool = Orm::pool().expect("MySQL pool");
    sqlx::query("DROP TABLE IF EXISTS rullst_audits")
        .execute(pool)
        .await
        .expect("drop the audit table");
    sqlx::query(
        "CREATE TABLE rullst_audits (id INT AUTO_INCREMENT PRIMARY KEY, \
         model_type VARCHAR(255) NOT NULL, model_id INT NOT NULL, event VARCHAR(50) NOT NULL, \
         old_values LONGTEXT, new_values LONGTEXT, \
         created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)",
    )
    .execute(pool)
    .await
    .expect("create a v1 audit table");

    rullst_orm::audit::create_audit_table()
        .await
        .expect("upgrade the v1 audit table");
    // old_values, new_values and the added restore_patch; the other added
    // columns keep their own types.
    assert_eq!(
        count_columns("rullst_audits", "DATA_TYPE = 'longtext'").await,
        3
    );
    assert_eq!(
        count_columns(
            "rullst_audits",
            "COLUMN_NAME = 'restore_patch' AND DATA_TYPE = 'longtext'"
        )
        .await,
        1
    );
    assert_eq!(
        count_columns(
            "rullst_audits",
            "COLUMN_NAME = 'actor_kind' AND DATA_TYPE = 'varchar'"
        )
        .await,
        1
    );
}

async fn exercise_native_enum() {
    Schema::create("mysql_enum_accounts", |table: &mut Blueprint| {
        table.id();
        table.native_enum::<AccountStatus>("status").not_null();
    })
    .await
    .expect("MySQL inline enum schema should be created");

    let pool = Orm::pool().expect("MySQL pool");
    let mut connection = pool.acquire().await.expect("MySQL connection");
    sqlx::query("SET SESSION sql_mode = 'STRICT_ALL_TABLES'")
        .execute(&mut *connection)
        .await
        .expect("strict MySQL enum validation should be enabled");
    sqlx::query("INSERT INTO mysql_enum_accounts (status) VALUES (?)")
        .bind(AccountStatus::AwaitingReview)
        .execute(&mut *connection)
        .await
        .expect("MySQL enum should encode");
    let stored = sqlx::query_scalar::<_, AccountStatus>(
        "SELECT status FROM mysql_enum_accounts WHERE id = 1",
    )
    .fetch_one(&mut *connection)
    .await
    .expect("MySQL enum should decode");
    assert_eq!(stored, AccountStatus::AwaitingReview);

    let invalid = sqlx::query("INSERT INTO mysql_enum_accounts (status) VALUES (?)")
        .bind("retired")
        .execute(&mut *connection)
        .await;
    assert!(invalid.is_err(), "MySQL ENUM must reject unknown labels");
    drop(connection);

    Schema::drop_if_exists("mysql_enum_accounts")
        .await
        .expect("MySQL enum table should be dropped");
}
