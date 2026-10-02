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

/// MySQL/MariaDB `TEXT` holds 64 KiB, far below the 5 MiB audit payload
/// bound, so payload columns use `LONGTEXT` there.
const MYSQL_PAYLOAD_TYPE: &str = "LONGTEXT";

/// Existing MySQL/MariaDB payload columns that are still `TEXT`-sized.
const MYSQL_SMALL_PAYLOAD_COLUMNS: &str = "SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'rullst_audits' AND COLUMN_NAME IN ('old_values', 'new_values', 'restore_patch') AND DATA_TYPE IN ('tinytext', 'text')";

/// Creates or upgrades the bounded v2 audit table.
///
/// On MySQL/MariaDB the payload columns of a new table are `LONGTEXT`. An
/// existing table is never altered implicitly: when its payload columns are
/// still `TEXT` (64 KiB), a warning names the reviewed migration to apply.
#[cfg_attr(test, mutants::skip)]
pub async fn create_audit_table() -> Result<(), crate::Error> {
    let pool = Orm::try_pool()?;
    let driver = Orm::try_driver()?;
    sqlx::query(create_table_sql(driver)).execute(pool).await?;
    ensure_v2_columns(pool, driver).await?;
    if driver == "mysql" {
        let (small_columns,): (i64,) = sqlx::query_as(MYSQL_SMALL_PAYLOAD_COLUMNS)
            .fetch_one(pool)
            .await?;
        if small_columns > 0 {
            tracing::warn!(
                target: "rullst_orm",
                "rullst_audits payload columns are TEXT (64 KiB); audited writes with larger payloads fail. Apply: ALTER TABLE rullst_audits MODIFY old_values LONGTEXT, MODIFY new_values LONGTEXT, MODIFY restore_patch LONGTEXT"
            );
        }
    }
    Ok(())
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
            old_values LONGTEXT,
            new_values LONGTEXT,
            actor_kind VARCHAR(16) NOT NULL DEFAULT 'legacy',
            actor_id VARCHAR(255) NOT NULL DEFAULT 'unknown',
            tenant_key VARCHAR(512),
            correlation_id VARCHAR(255),
            reverted_audit_id INT,
            reason TEXT,
            format_version INT NOT NULL DEFAULT 2,
            restore_patch LONGTEXT,
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

async fn ensure_v2_columns(pool: &crate::RullstPool, driver: &str) -> Result<(), crate::Error> {
    for (column, definition) in V2_COLUMNS {
        if column_exists(pool, column).await {
            continue;
        }
        let definition = if driver == "mysql" && column == "restore_patch" {
            MYSQL_PAYLOAD_TYPE
        } else {
            definition
        };
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

#[cfg(test)]
mod tests {
    use super::create_table_sql;

    #[test]
    fn mysql_payload_columns_hold_the_bounded_audit_payload() {
        let ddl = create_table_sql("mysql");
        for column in ["old_values", "new_values", "restore_patch"] {
            assert!(ddl.contains(&format!("{column} LONGTEXT,")), "{column}");
        }
        for driver in ["postgres", "sqlite"] {
            assert!(!create_table_sql(driver).contains("LONGTEXT"), "{driver}");
        }
    }
}
