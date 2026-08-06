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

impl SchemaErrorInterceptor {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a SQL error message and generate a corrective SQL migration script
    pub fn diagnose_sql_error(&self, error_message: &str) -> Option<SuggestedMigration> {
        let err_lower = error_message.to_lowercase();

        // 1. Detect missing column error (e.g., Postgres: column "phone" of relation "users" does not exist)
        if err_lower.contains("column") && (err_lower.contains("does not exist") || err_lower.contains("no such column")) {
            let col_name = self.extract_quoted_name(&err_lower, "column").unwrap_or_else(|| "missing_column".to_string());
            let table_name = self.extract_quoted_name(&err_lower, "relation").unwrap_or_else(|| "target_table".to_string());

            return Some(SuggestedMigration {
                name: format!("add_{}_to_{}", col_name, table_name),
                sql_statement: format!("ALTER TABLE {} ADD COLUMN {} TEXT;", table_name, col_name),
                target_table: table_name,
                reason: format!("Detected missing column '{}' in query execution", col_name),
            });
        }

        // 2. Detect missing table error (e.g., relation "orders" does not exist or no such table: orders)
        if err_lower.contains("relation") || err_lower.contains("no such table") {
            let table_name = self.extract_quoted_name(&err_lower, "relation")
                .or_else(|| self.extract_quoted_name(&err_lower, "table"))
                .unwrap_or_else(|| "missing_table".to_string());

            return Some(SuggestedMigration {
                name: format!("create_{}_table", table_name),
                sql_statement: format!(
                    "CREATE TABLE {} (\n    id BIGSERIAL PRIMARY KEY,\n    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP\n);",
                    table_name
                ),
                target_table: table_name.clone(),
                reason: format!("Detected missing table '{}' in query execution", table_name),
            });
        }

        None
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
        assert!(m.sql_statement.contains("ALTER TABLE users ADD COLUMN phone TEXT;"));
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
    }
}
