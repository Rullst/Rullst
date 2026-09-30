//! SQL dialect details shared by the Nexus CRUD queries.

/// Escape character for `LIKE` patterns. `!` is an ordinary character in
/// SQLite, PostgreSQL and MySQL string literals, unlike `\`, which MySQL
/// treats as a string escape.
const LIKE_ESCAPE: char = '!';

/// Wraps a search query as a `%…%` pattern that matches `q` literally: `%`,
/// `_` and the escape character itself are escaped.
pub(crate) fn contains_pattern(q: &str) -> String {
    let mut pattern = String::with_capacity(q.len().saturating_add(2));
    pattern.push('%');
    for character in q.chars() {
        if matches!(character, '%' | '_' | LIKE_ESCAPE) {
            pattern.push(LIKE_ESCAPE);
        }
        pattern.push(character);
    }
    pattern.push('%');
    pattern
}

/// A case-insensitive `LIKE` predicate for one sanitized column.
///
/// PostgreSQL's `LIKE` is case-sensitive, so it uses `ILIKE`; SQLite's `LIKE`
/// folds ASCII case and MySQL/MariaDB follow the column collation.
pub(crate) fn search_predicate(column: &str, placeholder: &str, driver: &str) -> String {
    if driver == "postgres" {
        format!("CAST({column} AS TEXT) ILIKE {placeholder} ESCAPE '{LIKE_ESCAPE}'")
    } else {
        format!("{column} LIKE {placeholder} ESCAPE '{LIKE_ESCAPE}'")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_patterns_match_wildcards_literally() {
        assert_eq!(contains_pattern("alice"), "%alice%");
        assert_eq!(contains_pattern("50%"), "%50!%%");
        assert_eq!(contains_pattern("a_b"), "%a!_b%");
        assert_eq!(contains_pattern("wow!"), "%wow!!%");
        assert_eq!(contains_pattern(r"C:\path"), r"%C:\path%");
    }

    #[test]
    fn search_is_case_insensitive_on_postgres() {
        assert_eq!(
            search_predicate("title", "$1", "postgres"),
            "CAST(title AS TEXT) ILIKE $1 ESCAPE '!'"
        );
        assert_eq!(
            search_predicate("title", "?", "sqlite"),
            "title LIKE ? ESCAPE '!'"
        );
        assert_eq!(
            search_predicate("title", "?", "mysql"),
            "title LIKE ? ESCAPE '!'"
        );
    }
}
