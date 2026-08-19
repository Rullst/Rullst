use crate::Error;

/// Allowlist of SQL comparison/join operators accepted in raw clause builders.
pub const ALLOWED_OPERATORS: &[&str] = &["=", "!=", "<>", "<", ">", "<=", ">="];

/// Validates a SQL identifier (column or table name) to prevent SQL injection.
/// Allows alphanumeric characters, underscores, hyphens and a single dot
/// for qualified names like `table.column`.
pub fn validate_identifier(name: &str) -> Result<(), Error> {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return Err(Error::Internal(
            "SQL identifier cannot be empty".to_string(),
        ));
    }

    // Check maximum length
    if bytes.len() > 64 {
        return Err(Error::Internal(format!(
            "Invalid SQL identifier '{}': exceeds maximum length of 64 characters",
            name
        )));
    }

    if bytes[0] == b'.' || bytes[bytes.len() - 1] == b'.' {
        return Err(Error::Internal(format!(
            "Invalid SQL identifier '{}': must not start or end with a dot",
            name
        )));
    }

    let mut dot_count = 0;
    for &b in bytes {
        if b == b'.' {
            dot_count += 1;
            if dot_count > 1 {
                return Err(Error::Internal(format!(
                    "Invalid SQL identifier '{}': at most one dot is allowed",
                    name
                )));
            }
        } else if !b.is_ascii_alphanumeric() && b != b'_' && b != b'-' {
            return Err(Error::Internal(format!(
                "Invalid SQL identifier '{}': only alphanumeric characters, underscores, hyphens and dots are allowed",
                name
            )));
        }
    }

    Ok(())
}

/// Validates a table name to prevent SQL injection.
pub fn validate_table_name(table_name: &str) -> Result<(), Error> {
    if table_name.contains('.') {
        return Err(Error::Internal(format!(
            "Invalid table name '{}': dots are not allowed in table names",
            table_name
        )));
    }
    validate_identifier(table_name)
}
