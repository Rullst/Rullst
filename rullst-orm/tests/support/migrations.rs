//! Built-in migration runner contracts shared by the server matrices.

#![allow(dead_code)]

use rullst_orm::Orm;
use rullst_orm::schema::run_artisan_with_args;

fn artisan(command: &str) -> Vec<String> {
    vec!["artisan".to_string(), command.to_string()]
}

/// `status` and `migrate:rollback` on a database without its own
/// `migrations` table ignore a same-named table in another schema/database.
#[allow(dead_code)]
pub async fn exercise_foreign_migrations_table() {
    let pool = Orm::pool().expect("pool");
    let (create_namespace, create_table, drop_namespace) = match Orm::driver().expect("driver") {
        "postgres" => (
            "CREATE SCHEMA rullst_foreign_app",
            "CREATE TABLE rullst_foreign_app.migrations (id SERIAL PRIMARY KEY, migration VARCHAR(255) NOT NULL, batch INTEGER NOT NULL)",
            "DROP SCHEMA rullst_foreign_app CASCADE",
        ),
        "mysql" => (
            "CREATE DATABASE rullst_foreign_app",
            "CREATE TABLE rullst_foreign_app.migrations (id INT AUTO_INCREMENT PRIMARY KEY, migration VARCHAR(255) NOT NULL, batch INT NOT NULL)",
            "DROP DATABASE rullst_foreign_app",
        ),
        _ => return,
    };
    sqlx::query("DROP TABLE IF EXISTS migrations")
        .execute(pool)
        .await
        .expect("start without an own migrations table");
    sqlx::query(create_namespace)
        .execute(pool)
        .await
        .expect("create foreign schema/database");
    sqlx::query(create_table)
        .execute(pool)
        .await
        .expect("create foreign migrations table");
    sqlx::query(
        "INSERT INTO rullst_foreign_app.migrations (migration, batch) VALUES ('m1_foreign', 1)",
    )
    .execute(pool)
    .await
    .expect("record a foreign migration");

    run_artisan_with_args(&artisan("status"), Vec::new(), Vec::new())
        .await
        .expect("status must ignore another schema's migrations table");
    run_artisan_with_args(&artisan("migrate:rollback"), Vec::new(), Vec::new())
        .await
        .expect("rollback must ignore another schema's migrations table");

    sqlx::query(drop_namespace)
        .execute(pool)
        .await
        .expect("drop foreign schema/database");
}
