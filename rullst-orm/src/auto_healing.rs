use serde::{Deserialize, Serialize};

/// Corrective Migration proposed by the Auto-Healing Database Engine (Milestone 21)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedMigration {
    pub name: String,
    pub sql_statement: String,
    pub target_table: String,
    pub reason: String,
}

/// Interceptor analyzing SQLx schema errors and generating corrective migrations
#[derive(Debug, Default)]
pub struct SchemaErrorInterceptor;

/// SQL dialect inferred from the wording of a missing-table error.
#[derive(Clone, Copy)]
enum Dialect {
    Postgres,
    Sqlite,
    Mysql,
}

impl SchemaErrorInterceptor {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a SQL error message and generate a corrective SQL migration script.
    ///
    /// Only "missing column" and "missing table" errors produce a suggestion:
    /// PostgreSQL `column "c" ... does not exist` / `relation "t" does not
    /// exist`, SQLite `no such column: c` / `no such table: t` and MySQL
    /// `Unknown column 'c'` / `Table 'db.t' doesn't exist`. Constraint
    /// violations, `already exists` errors and other messages that merely
    /// mention a relation return `None`, as does a name that is not a plain
    /// SQL identifier. The `CREATE TABLE` suggestion uses the dialect of the
    /// error message.
    pub fn diagnose_sql_error(&self, error_message: &str) -> Option<SuggestedMigration> {
        let err_lower = error_message.to_lowercase();
        if err_lower.contains("already exists") {
            return None;
        }

        // 1. Missing column (e.g. Postgres: column "phone" of relation "users" does not exist)
        let missing_column = if err_lower.contains("no such column") {
            Some(
                self.extract_after(&err_lower, "no such column:")
                    .unwrap_or_else(|| "missing_column".to_string()),
            )
        } else if err_lower.contains("unknown column") {
            Some(
                self.extract_quoted_name(&err_lower, "unknown column")
                    .unwrap_or_else(|| "missing_column".to_string()),
            )
        } else if err_lower.contains("column") && err_lower.contains("does not exist") {
            Some(
                self.extract_quoted_name(&err_lower, "column")
                    .unwrap_or_else(|| "missing_column".to_string()),
            )
        } else {
            None
        };
        if let Some(col_name) = missing_column {
            let table_name = self
                .extract_quoted_name(&err_lower, "relation")
                .unwrap_or_else(|| "target_table".to_string());
            if !is_identifier(&col_name) || !is_identifier(&table_name) {
                return None;
            }

            return Some(SuggestedMigration {
                name: format!("add_{}_to_{}", col_name, table_name),
                sql_statement: format!("ALTER TABLE {} ADD COLUMN {} TEXT;", table_name, col_name),
                target_table: table_name,
                reason: format!("Detected missing column '{}' in query execution", col_name),
            });
        }

        // 2. Missing table (relation "orders" does not exist, no such table: orders,
        //    or Table 'shop.orders' doesn't exist)
        let (table_name, dialect) = if err_lower.contains("no such table") {
            (
                self.extract_after(&err_lower, "no such table:"),
                Dialect::Sqlite,
            )
        } else if err_lower.contains("relation") && err_lower.contains("does not exist") {
            (
                self.extract_quoted_name(&err_lower, "relation"),
                Dialect::Postgres,
            )
        } else if err_lower.contains("table") && err_lower.contains("doesn't exist") {
            let qualified = self.extract_quoted_name(&err_lower, "table");
            let table = qualified.map(|name| match name.rsplit_once('.') {
                Some((_, table)) => table.to_string(),
                None => name,
            });
            (table, Dialect::Mysql)
        } else {
            return None;
        };
        let table_name = table_name.unwrap_or_else(|| "missing_table".to_string());
        if !is_identifier(&table_name) {
            return None;
        }
        let columns = match dialect {
            Dialect::Postgres => {
                "id BIGSERIAL PRIMARY KEY,\n    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP"
            }
            Dialect::Sqlite => {
                "id INTEGER PRIMARY KEY AUTOINCREMENT,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP"
            }
            Dialect::Mysql => {
                "id BIGINT AUTO_INCREMENT PRIMARY KEY,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP"
            }
        };

        Some(SuggestedMigration {
            name: format!("create_{}_table", table_name),
            sql_statement: format!("CREATE TABLE {} (\n    {}\n);", table_name, columns),
            target_table: table_name.clone(),
            reason: format!("Detected missing table '{}' in query execution", table_name),
        })
    }

    fn extract_quoted_name(&self, text: &str, prefix: &str) -> Option<String> {
        if let Some(pos) = text.find(prefix) {
            let remainder = &text[pos + prefix.len()..];
            if let Some(start_quote) = remainder.find('"').or_else(|| remainder.find('\'')) {
                let quote_char = remainder.as_bytes()[start_quote] as char;
                let sub = &remainder[start_quote + 1..];
                if let Some(end_quote) = sub.find(quote_char) {
                    return Some(sub[..end_quote].to_string());
                }
            }
        }
        None
    }

    /// The unquoted name SQLite prints after `marker` (`no such table: t`),
    /// without a schema qualifier such as `main.`.
    fn extract_after(&self, text: &str, marker: &str) -> Option<String> {
        let start = text.find(marker)? + marker.len();
        let name = text[start..]
            .trim_start()
            .split(|character: char| {
                !(character.is_ascii_alphanumeric() || "_.".contains(character))
            })
            .next()?;
        let name = name.rsplit_once('.').map_or(name, |(_, name)| name);
        (!name.is_empty()).then(|| name.to_string())
    }
}

/// A plain SQL identifier: ASCII letters, digits and underscores, at most 64
/// bytes, so a suggested statement never carries other text from the error.
fn is_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnose_missing_column() {
        let interceptor = SchemaErrorInterceptor::new();
        let err = r#"db error: column "phone" of relation "users" does not exist"#;

        let migration = interceptor.diagnose_sql_error(err);
        assert!(migration.is_some());
        let m = migration.unwrap();
        assert_eq!(m.target_table, "users");
        assert!(
            m.sql_statement
                .contains("ALTER TABLE users ADD COLUMN phone TEXT;")
        );
    }

    #[test]
    fn test_diagnose_missing_table() {
        let interceptor = SchemaErrorInterceptor::new();
        let err = r#"db error: relation "orders" does not exist"#;

        let migration = interceptor.diagnose_sql_error(err);
        assert!(migration.is_some());
        let m = migration.unwrap();
        assert_eq!(m.target_table, "orders");
        assert!(m.sql_statement.contains("CREATE TABLE orders"));
        assert!(m.sql_statement.contains("BIGSERIAL"));
    }

    #[test]
    fn errors_that_only_mention_a_relation_are_not_missing_tables() {
        let interceptor = SchemaErrorInterceptor::new();
        for err in [
            r#"null value in column "email" of relation "users" violates not-null constraint"#,
            r#"relation "users" already exists"#,
            r#"column "phone" of relation "users" already exists"#,
            r#"duplicate key value violates unique constraint "users_email_key""#,
            r#"insert or update on table "posts" violates foreign key constraint "posts_user_id_fkey""#,
            "permission denied for relation users",
            "UNIQUE constraint failed: users.email",
            "table users already exists",
        ] {
            assert!(
                interceptor.diagnose_sql_error(err).is_none(),
                "{err} must not produce a suggestion"
            );
        }
    }

    #[test]
    fn missing_tables_use_the_dialect_of_the_error() {
        let interceptor = SchemaErrorInterceptor::new();
        let sqlite = interceptor
            .diagnose_sql_error(
                "error returned from database: (code: 1) no such table: main.orders",
            )
            .expect("SQLite missing table");
        assert_eq!(sqlite.target_table, "orders");
        assert!(
            sqlite
                .sql_statement
                .contains("INTEGER PRIMARY KEY AUTOINCREMENT")
        );
        assert!(!sqlite.sql_statement.contains("BIGSERIAL"));

        let mysql = interceptor
            .diagnose_sql_error("1146 (42S02): Table 'shop.orders' doesn't exist")
            .expect("MySQL missing table");
        assert_eq!(mysql.target_table, "orders");
        assert!(mysql.sql_statement.contains("AUTO_INCREMENT"));

        let column = interceptor
            .diagnose_sql_error("1054 (42S22): Unknown column 'phone' in 'field list'")
            .expect("MySQL missing column");
        assert!(column.sql_statement.contains("ADD COLUMN phone TEXT"));
        let column = interceptor
            .diagnose_sql_error("no such column: phone")
            .expect("SQLite missing column");
        assert!(column.sql_statement.contains("ADD COLUMN phone TEXT"));
    }

    #[test]
    fn names_that_are_not_identifiers_produce_no_statement() {
        let interceptor = SchemaErrorInterceptor::new();
        assert!(
            interceptor
                .diagnose_sql_error(r#"relation "orders; drop table users" does not exist"#)
                .is_none()
        );
    }
}
