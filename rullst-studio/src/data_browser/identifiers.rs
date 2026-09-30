//! SQL identifier validation and quoting for Studio's dynamic statements.

/// Sanitize table and column names to prevent SQL injections in dynamic queries
pub fn sanitize_identifier(id: &str) -> String {
    let mut res = String::with_capacity(64);
    for c in id.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            if res.len() == 64 {
                break;
            }
            res.push(c);
        }
    }
    res
}

/// Whether an identifier is accepted by Studio's deliberately narrow dynamic-SQL boundary.
pub fn is_safe_identifier(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && sanitize_identifier(id) == id
}

pub fn quote_table_name(driver: &str, clean_table: &str) -> String {
    if driver == "mysql" {
        format!("`{}`", clean_table)
    } else {
        format!("\"{}\"", clean_table)
    }
}

/// Quotes a table for data statements. Studio inspects PostgreSQL tables in
/// the `public` schema, so their data statements name that schema too: an
/// unqualified name follows `search_path` (by default `"$user", public`) and
/// could reach a same-named table in another schema.
pub(crate) fn qualified_table_name(driver: &str, clean_table: &str) -> String {
    if driver == "postgres" {
        format!("\"public\".\"{clean_table}\"")
    } else {
        quote_table_name(driver, clean_table)
    }
}

/// Helper to build a search clause taking driver syntax into account
pub fn build_search_clause(driver: &str, col: &str) -> String {
    if driver == "postgres" {
        format!("CAST(\"{}\" AS TEXT) ILIKE ", sanitize_identifier(col))
    } else if driver == "mysql" {
        format!("CAST(`{}` AS CHAR) LIKE ", sanitize_identifier(col))
    } else {
        format!("\"{}\" LIKE ", sanitize_identifier(col))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_data_statements_name_the_inspected_schema() {
        assert_eq!(
            qualified_table_name("postgres", "users"),
            "\"public\".\"users\""
        );
        assert_eq!(qualified_table_name("sqlite", "users"), "\"users\"");
        assert_eq!(qualified_table_name("mysql", "users"), "`users`");
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    #[kani::unwind(5)]
    fn proof_sanitize_identifier_length_bound() {
        let id: [u8; 4] = kani::any();
        if let Ok(s) = std::str::from_utf8(&id) {
            let clean = sanitize_identifier(s);
            assert!(clean.len() <= 64);
        }
    }
}
