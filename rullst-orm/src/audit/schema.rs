use crate::Orm;

const V2_COLUMNS: [(&str, &str); 8] = [
    ("actor_kind", "VARCHAR(16) NOT NULL DEFAULT 'legacy'"),
    ("actor_id", "VARCHAR(255) NOT NULL DEFAULT 'unknown'"),
    ("tenant_key", "VARCHAR(512)"),
    ("correlation_id", "VARCHAR(255)"),
    ("reverted_audit_id", "INT"),
    ("reason", "TEXT"),
    ("format_version", "INT NOT NULL DEFAULT 1"),
    ("restore_patch", "TEXT"),
];

/// Creates or upgrades the bounded v2 audit table.
#[cfg_attr(test, mutants::skip)]
pub async fn create_audit_table() -> Result<(), crate::Error> {
    let pool = Orm::try_pool()?;
    let driver = Orm::try_driver()?;
    sqlx::query(create_table_sql(driver)).execute(pool).await?;
    ensure_v2_columns(pool).await
}

fn create_table_sql(driver: &str) -> &'static str {
    if driver == "postgres" {
        r#"
        CREATE TABLE IF NOT EXISTS rullst_audits (
            id SERIAL PRIMARY KEY,
            model_type VARCHAR(255) NOT NULL,
            model_id INT NOT NULL,
            event VARCHAR(50) NOT NULL,
            old_values TEXT,
            new_values TEXT,
            actor_kind VARCHAR(16) NOT NULL DEFAULT 'legacy',
            actor_id VARCHAR(255) NOT NULL DEFAULT 'unknown',
            tenant_key VARCHAR(512),
            correlation_id VARCHAR(255),
            reverted_audit_id INT,
            reason TEXT,
            format_version INT NOT NULL DEFAULT 2,
            restore_patch TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#
    } else if driver == "mysql" {
        r#"
        CREATE TABLE IF NOT EXISTS rullst_audits (
            id INT AUTO_INCREMENT PRIMARY KEY,
            model_type VARCHAR(255) NOT NULL,
            model_id INT NOT NULL,
            event VARCHAR(50) NOT NULL,
            old_values TEXT,
            new_values TEXT,
            actor_kind VARCHAR(16) NOT NULL DEFAULT 'legacy',
            actor_id VARCHAR(255) NOT NULL DEFAULT 'unknown',
            tenant_key VARCHAR(512),
            correlation_id VARCHAR(255),
            reverted_audit_id INT,
            reason TEXT,
            format_version INT NOT NULL DEFAULT 2,
            restore_patch TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#
    } else {
        r#"
        CREATE TABLE IF NOT EXISTS rullst_audits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            model_type TEXT NOT NULL,
            model_id INTEGER NOT NULL,
            event TEXT NOT NULL,
            old_values TEXT,
            new_values TEXT,
            actor_kind TEXT NOT NULL DEFAULT 'legacy',
            actor_id TEXT NOT NULL DEFAULT 'unknown',
            tenant_key TEXT,
            correlation_id TEXT,
            reverted_audit_id INTEGER,
            reason TEXT,
            format_version INTEGER NOT NULL DEFAULT 2,
            restore_patch TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#
    }
}

async fn ensure_v2_columns(pool: &crate::RullstPool) -> Result<(), crate::Error> {
    for (column, definition) in V2_COLUMNS {
        if column_exists(pool, column).await {
            continue;
        }
        let migration = format!("ALTER TABLE rullst_audits ADD COLUMN {column} {definition}");
        let result = sqlx::query(sqlx::AssertSqlSafe(migration.as_str()))
            .execute(pool)
            .await;
        if let Err(error) = result
            && !column_exists(pool, column).await
        {
            return Err(error.into());
        }
    }
    Ok(())
}

async fn column_exists(pool: &crate::RullstPool, column: &str) -> bool {
    let probe = format!("SELECT {column} FROM rullst_audits WHERE 1 = 0");
    sqlx::query(sqlx::AssertSqlSafe(probe.as_str()))
        .execute(pool)
        .await
        .is_ok()
}
