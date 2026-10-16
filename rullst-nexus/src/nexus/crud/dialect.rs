//! SQL dialect details shared by the Nexus CRUD queries.

use rullst_orm::RullstValue;

use super::input::has_offset;
use super::query::sanitize_identifier;
use crate::nexus::types::{FieldKind, FieldMeta};

/// A query built from the ORM pool's database driver.
pub(crate) type NexusQuery<'q> = rullst_orm::_sqlx::query::Query<
    'q,
    rullst_orm::RullstDatabase,
    <rullst_orm::RullstDatabase as rullst_orm::_sqlx::Database>::Arguments,
>;

/// A positional parameter marker for the driver.
pub(crate) fn placeholder(position: usize, driver: &str) -> String {
    if driver == "postgres" {
        format!("${position}")
    } else {
        "?".to_string()
    }
}

/// SQL that writes one validated form value.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ValueSql {
    /// An expression around the next placeholder; the value is bound there.
    Bind(String),
    /// A literal from a closed set; nothing is bound.
    Literal(&'static str),
}

/// Chooses how a validated value of `kind` is written at bind `position`.
///
/// Form values are bound as text. SQLite and MySQL convert text on
/// assignment, but PostgreSQL has no assignment cast from `text` to numeric or
/// Boolean columns, so there:
/// - numbers go through `NUMERIC`, which converts exactly and assigns to every
///   integer, floating-point, numeric and text column;
/// - a relation value that is a canonical integer (or NULL) goes through
///   `BIGINT`, which assigns to integer and text key columns alike;
/// - a Boolean (normalized to `0`/`1`) is an untyped literal, the only form
///   both the `INTEGER` columns of `Blueprint::boolean` and native `BOOLEAN`
///   columns accept;
/// - a date, time or date-time (and its NULL) is cast to `DATE`, `TIME`,
///   `TIMESTAMP` or `TIMESTAMPTZ`, which native columns need and text columns
///   accept by assignment; only a [`FieldKind::DateTime`] value with an
///   offset, which only a text column can hold, stays text.
///
/// Other kinds stay text, matching Rullst's text and JSON columns.
pub(crate) fn write_value_sql(
    kind: &FieldKind,
    value: Option<&str>,
    position: usize,
    driver: &str,
) -> ValueSql {
    let marker = placeholder(position, driver);
    if driver != "postgres" {
        return ValueSql::Bind(marker);
    }
    match (kind, value) {
        (FieldKind::Boolean, Some("0")) => ValueSql::Literal("'0'"),
        (FieldKind::Boolean, Some("1")) => ValueSql::Literal("'1'"),
        (FieldKind::Number | FieldKind::Integer { .. }, _) => {
            ValueSql::Bind(format!("CAST({marker} AS NUMERIC)"))
        }
        (FieldKind::Date, _) => ValueSql::Bind(format!("CAST({marker} AS DATE)")),
        (FieldKind::Time, _) => ValueSql::Bind(format!("CAST({marker} AS TIME)")),
        (FieldKind::DateTimeUtc, _) => ValueSql::Bind(format!("CAST({marker} AS TIMESTAMPTZ)")),
        (FieldKind::DateTime, Some(value)) if has_offset(value) => ValueSql::Bind(marker),
        (FieldKind::DateTime, _) => ValueSql::Bind(format!("CAST({marker} AS TIMESTAMP)")),
        (FieldKind::ForeignKey { .. }, None) => ValueSql::Bind(format!("CAST({marker} AS BIGINT)")),
        (FieldKind::ForeignKey { .. }, Some(value))
            if value
                .parse::<i64>()
                .is_ok_and(|parsed| parsed.to_string() == value) =>
        {
            ValueSql::Bind(format!("CAST({marker} AS BIGINT)"))
        }
        _ => ValueSql::Bind(marker),
    }
}

/// The select-list entry that reads `field`. PostgreSQL and MySQL/MariaDB
/// date, time and date-time columns are read as text: their native `DATE`,
/// `TIME`, `TIMESTAMP` and `TIMESTAMPTZ` values do not decode as `String`, and
/// the SQLx `Any` pool cannot decode them at all. A text column reads
/// unchanged.
pub(crate) fn select_column(field: &FieldMeta, driver: &str) -> String {
    let column = sanitize_identifier(field.name);
    match (&field.kind, driver) {
        (
            FieldKind::Date | FieldKind::DateTime | FieldKind::Time | FieldKind::DateTimeUtc,
            "postgres",
        ) => format!("CAST({column} AS TEXT) AS {column}"),
        (
            FieldKind::Date | FieldKind::DateTime | FieldKind::Time | FieldKind::DateTimeUtc,
            "mysql",
        ) => format!("CAST({column} AS CHAR) AS {column}"),
        _ => column,
    }
}

/// The value bound for a typed date or time on `driver`. MySQL and MariaDB
/// `DATETIME` columns hold UTC without a zone (MariaDB rejects an offset in a
/// date-time literal), so a UTC instant is bound there as its UTC wall time.
pub(crate) fn driver_value(value: RullstValue, driver: &str) -> RullstValue {
    match value {
        RullstValue::DateTime(instant) if driver == "mysql" => {
            RullstValue::NaiveDateTime(instant.naive_utc())
        }
        other => other,
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

    #[test]
    fn integer_columns_are_written_as_numeric_on_postgres() {
        let integer = FieldKind::Integer {
            min: 0,
            max: i64::MAX,
        };
        assert_eq!(
            write_value_sql(&integer, Some("7"), 2, "postgres"),
            ValueSql::Bind("CAST($2 AS NUMERIC)".into())
        );
    }

    #[test]
    fn date_columns_are_selected_as_text_on_postgres_and_mysql() {
        let created = crate::nexus::FieldMeta::new("created_at", "Created", FieldKind::DateTime);
        let born = crate::nexus::FieldMeta::new("born_on", "Born", FieldKind::Date);
        let title = crate::nexus::FieldMeta::new("title", "Title", FieldKind::Text);
        assert_eq!(
            select_column(&created, "postgres"),
            "CAST(created_at AS TEXT) AS created_at"
        );
        assert_eq!(
            select_column(&born, "postgres"),
            "CAST(born_on AS TEXT) AS born_on"
        );
        assert_eq!(
            select_column(&created, "mysql"),
            "CAST(created_at AS CHAR) AS created_at"
        );
        assert_eq!(
            select_column(&born, "mysql"),
            "CAST(born_on AS CHAR) AS born_on"
        );
        assert_eq!(select_column(&created, "sqlite"), "created_at");
        assert_eq!(select_column(&title, "postgres"), "title");
        assert_eq!(select_column(&title, "mysql"), "title");
    }

    #[test]
    fn postgres_writes_cast_numbers_relations_and_booleans() {
        let relation = FieldKind::ForeignKey {
            table: "teams",
            label_col: "name",
        };
        let cases = [
            (
                FieldKind::Number,
                Some("4.5"),
                ValueSql::Bind("CAST($3 AS NUMERIC)".into()),
            ),
            (
                FieldKind::Number,
                None,
                ValueSql::Bind("CAST($3 AS NUMERIC)".into()),
            ),
            (
                relation.clone(),
                Some("42"),
                ValueSql::Bind("CAST($3 AS BIGINT)".into()),
            ),
            (
                relation.clone(),
                None,
                ValueSql::Bind("CAST($3 AS BIGINT)".into()),
            ),
            (relation.clone(), Some("042"), ValueSql::Bind("$3".into())),
            (relation, Some("team-a"), ValueSql::Bind("$3".into())),
            (FieldKind::Boolean, Some("1"), ValueSql::Literal("'1'")),
            (FieldKind::Boolean, Some("0"), ValueSql::Literal("'0'")),
            (FieldKind::Text, Some("1"), ValueSql::Bind("$3".into())),
        ];
        for (kind, value, expected) in cases {
            assert_eq!(
                write_value_sql(&kind, value, 3, "postgres"),
                expected,
                "{kind:?}"
            );
        }
        for driver in ["sqlite", "mysql"] {
            for kind in [FieldKind::Number, FieldKind::Boolean] {
                assert_eq!(
                    write_value_sql(&kind, Some("1"), 3, driver),
                    ValueSql::Bind("?".into())
                );
            }
        }
    }

    #[test]
    fn postgres_writes_cast_dates_and_times_for_native_columns() {
        let cases = [
            (FieldKind::Date, Some("2026-10-15"), "CAST($4 AS DATE)"),
            (FieldKind::Date, None, "CAST($4 AS DATE)"),
            (FieldKind::Time, Some("09:30:00"), "CAST($4 AS TIME)"),
            (FieldKind::Time, None, "CAST($4 AS TIME)"),
            (
                FieldKind::DateTime,
                Some("2026-10-15 09:30:00"),
                "CAST($4 AS TIMESTAMP)",
            ),
            (FieldKind::DateTime, None, "CAST($4 AS TIMESTAMP)"),
            // Only a text column holds a date-time with an offset.
            (FieldKind::DateTime, Some("2026-10-15T09:30:00Z"), "$4"),
            (FieldKind::DateTime, Some("2026-10-15 09:30-03:00"), "$4"),
            (
                FieldKind::DateTimeUtc,
                Some("2026-10-15T09:30:00+00:00"),
                "CAST($4 AS TIMESTAMPTZ)",
            ),
            (FieldKind::DateTimeUtc, None, "CAST($4 AS TIMESTAMPTZ)"),
        ];
        for (kind, value, expected) in cases {
            assert_eq!(
                write_value_sql(&kind, value, 4, "postgres"),
                ValueSql::Bind(expected.into()),
                "{kind:?} {value:?}"
            );
            for driver in ["sqlite", "mysql"] {
                assert_eq!(
                    write_value_sql(&kind, value, 4, driver),
                    ValueSql::Bind("?".into())
                );
            }
        }
    }

    #[test]
    fn time_columns_are_selected_as_text_on_postgres_and_mysql() {
        let opens = crate::nexus::FieldMeta::new("opens_at", "Opens", FieldKind::Time);
        let paid = crate::nexus::FieldMeta::new("paid_at", "Paid", FieldKind::DateTimeUtc);
        assert_eq!(
            select_column(&opens, "postgres"),
            "CAST(opens_at AS TEXT) AS opens_at"
        );
        assert_eq!(
            select_column(&paid, "mysql"),
            "CAST(paid_at AS CHAR) AS paid_at"
        );
        assert_eq!(select_column(&paid, "sqlite"), "paid_at");
    }

    #[test]
    fn mysql_binds_a_utc_instant_as_its_utc_wall_time() {
        let instant = rullst_orm::chrono::DateTime::parse_from_rfc3339("2026-10-15T09:30:00.5Z")
            .expect("valid instant")
            .with_timezone(&rullst_orm::chrono::Utc);
        assert_eq!(
            format!(
                "{:?}",
                driver_value(RullstValue::DateTime(instant), "mysql")
            ),
            "NaiveDateTime(2026-10-15T09:30:00.500)"
        );
        for driver in ["postgres", "sqlite"] {
            assert_eq!(
                format!("{:?}", driver_value(RullstValue::DateTime(instant), driver)),
                "DateTime(2026-10-15T09:30:00.500Z)"
            );
        }
        assert_eq!(
            format!(
                "{:?}",
                driver_value(RullstValue::String("x".into()), "mysql")
            ),
            "String(\"x\")"
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
