//! Logical column types whose DDL spelling is chosen per driver when a
//! [`Blueprint`](super::Blueprint) is built.

use super::column::ColumnDefault;
use crate::Error;

/// SQLite default of a `timestamp_tz()` column: the RFC 3339 UTC text that
/// SQLx writes for a `DateTime<Utc>`, so stored values sort as text.
const SQLITE_UTC_NOW: &str = "(strftime('%Y-%m-%dT%H:%M:%f+00:00', 'now'))";
/// PostgreSQL default of a zone-less column: the current UTC wall time.
const POSTGRES_UTC_NOW: &str = "(CURRENT_TIMESTAMP AT TIME ZONE 'UTC')";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DriverType {
    /// An `f64` column: `REAL` is only single precision on PostgreSQL.
    Double,
    /// A `bool` column: PostgreSQL has no implicit integer/boolean casts.
    Boolean,
    /// A calendar date (`chrono::NaiveDate`).
    Date,
    /// A time of day (`chrono::NaiveTime`).
    Time,
    /// A date and time without a zone (`chrono::NaiveDateTime`).
    Timestamp,
    /// A UTC instant (`chrono::DateTime<Utc>`).
    TimestampTz,
    /// `timestamps()`: a [`DriverType::TimestampTz`] with a strict driver, and
    /// the alpha.1 `TEXT` column with the SQLx `Any` pool, which cannot read
    /// date/time columns.
    AutoTimestamp,
}

impl DriverType {
    /// The type recorded in [`Column::col_type`](super::Column::col_type)
    /// when the column is added.
    pub(super) fn portable(self) -> &'static str {
        match self {
            Self::Double => "REAL",
            Self::Boolean => "INTEGER",
            Self::Date => "DATE",
            Self::Time => "TIME",
            Self::Timestamp => "TIMESTAMP",
            Self::TimestampTz | Self::AutoTimestamp => "TIMESTAMPTZ",
        }
    }

    /// Whether the column holds a native date/time type on PostgreSQL and
    /// MySQL/MariaDB, which the SQLx `Any` pool cannot decode.
    pub(super) fn is_native_temporal(self) -> bool {
        matches!(
            self,
            Self::Date | Self::Time | Self::Timestamp | Self::TimestampTz
        )
    }

    /// The kind actually rendered: `timestamps()` keeps `TEXT` on the `Any` pool.
    fn resolved(self, any_pool: bool) -> Option<Self> {
        match (self, any_pool) {
            (Self::AutoTimestamp, true) => None,
            (Self::AutoTimestamp, false) => Some(Self::TimestampTz),
            (kind, _) => Some(kind),
        }
    }

    pub(super) fn for_driver(self, driver: &str, any_pool: bool) -> &'static str {
        let Some(kind) = self.resolved(any_pool) else {
            return "TEXT";
        };
        match (kind, driver) {
            (Self::Double, "postgres") => "DOUBLE PRECISION",
            (Self::Double, "mysql") => "DOUBLE",
            (Self::Double, _) => "REAL",
            (Self::Boolean, "postgres") => "BOOLEAN",
            (Self::Boolean, _) => "INTEGER",
            (Self::Date, "postgres" | "mysql") => "DATE",
            (Self::Time, "postgres") => "TIME",
            (Self::Time, "mysql") => "TIME(6)",
            (Self::Timestamp, "postgres") => "TIMESTAMP",
            (Self::TimestampTz, "postgres") => "TIMESTAMPTZ",
            (Self::Timestamp | Self::TimestampTz, "mysql") => "DATETIME(6)",
            // SQLite stores ISO-8601 text; a declared DATE/DATETIME type
            // would also break SQLx `Any`, which cannot map those types.
            _ => "TEXT",
        }
    }

    /// Renders a column default, spelling `0`/`1` as a PostgreSQL boolean
    /// and `CURRENT_TIMESTAMP` in the column's own date/time form.
    pub(super) fn default_sql(
        self,
        driver: &str,
        any_pool: bool,
        default: &ColumnDefault,
        column: &str,
    ) -> Result<String, Error> {
        let Some(kind) = self.resolved(any_pool) else {
            return Ok(default.to_sql());
        };
        match (kind, driver, default) {
            (Self::Boolean, "postgres", ColumnDefault::Integer(0)) => Ok("FALSE".to_string()),
            (Self::Boolean, "postgres", ColumnDefault::Integer(1)) => Ok("TRUE".to_string()),
            (Self::Boolean, "postgres", ColumnDefault::Integer(_)) => Err(Error::Validation(
                format!("DEFAULT of boolean column `{column}` must be 0 or 1"),
            )),
            (kind, driver, ColumnDefault::CurrentTimestamp) if kind.is_native_temporal() => {
                Ok(current_time_sql(kind, driver).to_string())
            }
            _ => Ok(default.to_sql()),
        }
    }
}

/// `ColumnDefault::CurrentTimestamp` of a date/time column. Values are UTC:
/// PostgreSQL converts the zone-less forms, and SQLx sets MySQL/MariaDB
/// sessions to `+00:00`.
fn current_time_sql(kind: DriverType, driver: &str) -> &'static str {
    match (kind, driver) {
        (DriverType::TimestampTz, "postgres") => "CURRENT_TIMESTAMP",
        (_, "postgres") => POSTGRES_UTC_NOW,
        (DriverType::Date, "mysql") => "(CURRENT_DATE)",
        (DriverType::Time, "mysql") => "(CURRENT_TIME(6))",
        (_, "mysql") => "CURRENT_TIMESTAMP(6)",
        (DriverType::Date, _) => "CURRENT_DATE",
        (DriverType::Time, _) => "CURRENT_TIME",
        (DriverType::TimestampTz, _) => SQLITE_UTC_NOW,
        _ => "CURRENT_TIMESTAMP",
    }
}

/// MySQL accepts a DEFAULT on BLOB, TEXT, GEOMETRY and JSON columns only as a
/// parenthesized expression, even for a literal value (MySQL 8.0.13+), and it
/// cannot index those columns without a prefix length. MariaDB 10.2.1+
/// accepts the same expression form.
pub(super) fn mysql_blob_like(col_type: &str) -> bool {
    let base = col_type
        .split(|character: char| !character.is_ascii_alphanumeric())
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(
        base.as_str(),
        "TEXT"
            | "TINYTEXT"
            | "MEDIUMTEXT"
            | "LONGTEXT"
            | "BLOB"
            | "TINYBLOB"
            | "MEDIUMBLOB"
            | "LONGBLOB"
            | "JSON"
            | "GEOMETRY"
    )
}
