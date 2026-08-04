//! Intent-Based Modeling (Auto-generating migrations from Rust doc comments).

/// Intent analyzer for Rust model doc comments.
pub struct IntentAnalyzer;

impl IntentAnalyzer {
    /// Parses Rust source code doc-comments (e.g. `/// @index(email)`) and returns recommended migration SQL statements.
    pub fn analyze_doc_comments(code: &str, table_name: &str) -> Vec<String> {
        let mut migrations = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();
            if let Some(start) = trimmed.find("@index(") {
                let rest = &trimmed[start + "@index(".len()..];
                if let Some(end) = rest.find(')') {
                    let col = &rest[..end].trim();
                    let sql = format!("CREATE INDEX IF NOT EXISTS idx_{}_{} ON {}({});", table_name, col, table_name, col);
                    migrations.push(sql);
                }
            }
        }

        migrations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_doc_comments() {
        let code = r#"
        /// User Entity Model
        /// @index(email)
        /// @index(created_at)
        pub struct User {
            pub email: String,
        }
        "#;

        let sqls = IntentAnalyzer::analyze_doc_comments(code, "users");
        assert_eq!(sqls.len(), 2);
        assert_eq!(sqls[0], "CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);");
        assert_eq!(sqls[1], "CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at);");
    }
}
