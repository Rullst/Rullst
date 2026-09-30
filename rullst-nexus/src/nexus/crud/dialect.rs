//! SQL dialect details shared by the Nexus CRUD queries.

use std::borrow::Cow;

use super::query::sanitize_identifier;
use crate::nexus::types::{FieldKind, RegistryEntry};

/// A query built from the ORM pool's database driver.
pub(crate) type NexusQuery<'q> = rullst_orm::_sqlx::query::Query<
    'q,
    rullst_orm::RullstDatabase,
    <rullst_orm::RullstDatabase as rullst_orm::_sqlx::Database>::Arguments,
>;

/// A record key from a URL or batch form, typed by the registered primary key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordKey<'a> {
    /// A `number` or relation key: bound as a 64-bit integer.
    Integer(i64),
    /// Any other key kind: compared as text, so a text key such as `1001`
    /// is never bound as an integer.
    Text(&'a str),
}

impl<'a> RecordKey<'a> {
    /// Parses `id` as the model's primary-key kind requires.
    ///
    /// A numeric key must be a canonical integer. `+1`, `01` or `1e3` would
    /// otherwise reach a record whose stored key is spelled differently (Rust
    /// integer parsing and SQLite numeric affinity both accept them), and the
    /// audit would name a key that differs from the changed record. `None`
    /// means `id` cannot name a record of this model.
    pub(crate) fn parse(entry: &RegistryEntry, id: &'a str) -> Option<Self> {
        let canonical = id
            .parse::<i64>()
            .ok()
            .filter(|value| value.to_string() == id);
        let integer_key = entry
            .fields
            .iter()
            .find(|field| field.name == entry.pk)
            .map(|field| matches!(field.kind, FieldKind::Number | FieldKind::ForeignKey { .. }));
        match (integer_key, canonical) {
            (Some(true) | None, Some(value)) => Some(Self::Integer(value)),
            (Some(true), None) => None,
            (Some(false) | None, None) | (Some(false), Some(_)) => Some(Self::Text(id)),
        }
    }

    /// The canonical key text, as recorded by the audit.
    pub(crate) fn text(self) -> Cow<'a, str> {
        match self {
            Self::Integer(value) => Cow::Owned(value.to_string()),
            Self::Text(value) => Cow::Borrowed(value),
        }
    }

    /// Binds the key as its registered kind.
    pub(crate) fn bind<'q>(self, query: NexusQuery<'q>) -> NexusQuery<'q> {
        match self {
            Self::Integer(value) => query.bind(value),
            Self::Text(value) => query.bind(value.to_owned()),
        }
    }
}

/// The exact tenant scope `column = placeholder`.
///
/// MySQL and MariaDB default collations compare case-insensitively (and
/// ignore trailing spaces), which would equate the distinct Core tenants
/// `Acme` and `acme`; there both sides are compared as binary strings.
pub(crate) fn tenant_predicate(column: &str, placeholder: &str, driver: &str) -> String {
    let column = sanitize_identifier(column);
    if driver == "mysql" {
        format!("CAST({column} AS BINARY) = CAST({placeholder} AS BINARY)")
    } else {
        format!("{column} = {placeholder}")
    }
}

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

    fn entry(pk_kind: FieldKind) -> RegistryEntry {
        RegistryEntry {
            table: "records",
            label: "Records",
            icon: "R",
            pk: "id",
            tenant_column: None,
            fields: vec![crate::nexus::FieldMeta::new("id", "ID", pk_kind)],
        }
    }

    #[test]
    fn numeric_keys_must_be_canonical_integers() {
        let numeric = entry(FieldKind::Number);
        assert_eq!(
            RecordKey::parse(&numeric, "1000"),
            Some(RecordKey::Integer(1000))
        );
        assert_eq!(
            RecordKey::parse(&numeric, "-7"),
            Some(RecordKey::Integer(-7))
        );
        for spelling in ["+1000", "01000", "1e3", "1000.0", " 1000", "abc", ""] {
            assert_eq!(RecordKey::parse(&numeric, spelling), None, "{spelling:?}");
        }
        assert_eq!(
            RecordKey::parse(&numeric, "1000")
                .map(RecordKey::text)
                .as_deref(),
            Some("1000")
        );
    }

    #[test]
    fn text_keys_are_never_bound_as_integers() {
        let text = entry(FieldKind::Text);
        assert_eq!(
            RecordKey::parse(&text, "1001"),
            Some(RecordKey::Text("1001"))
        );
        assert_eq!(RecordKey::parse(&text, "new"), Some(RecordKey::Text("new")));

        // An entry whose key is not registered keeps the integer fallback.
        let mut unregistered = entry(FieldKind::Text);
        unregistered.pk = "uuid";
        assert_eq!(
            RecordKey::parse(&unregistered, "12"),
            Some(RecordKey::Integer(12))
        );
        assert_eq!(
            RecordKey::parse(&unregistered, "012"),
            Some(RecordKey::Text("012"))
        );
    }

    #[test]
    fn tenant_scope_is_byte_exact_on_mysql() {
        assert_eq!(
            tenant_predicate("tenant_id", "?", "mysql"),
            "CAST(tenant_id AS BINARY) = CAST(? AS BINARY)"
        );
        assert_eq!(
            tenant_predicate("tenant_id", "$3", "postgres"),
            "tenant_id = $3"
        );
        assert_eq!(tenant_predicate("tenant;id", "?", "sqlite"), "tenantid = ?");
    }

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
