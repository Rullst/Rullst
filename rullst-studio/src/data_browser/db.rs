//! Studio Database Inspection & Query Helpers

use serde::Deserialize;
use sqlx::{QueryBuilder, Row};
use std::fmt::Write;

pub(crate) use super::identifiers::qualified_table_name;
pub use super::identifiers::{
    build_search_clause, is_safe_identifier, quote_table_name, sanitize_identifier,
};
use super::limits::display_cell;
pub use super::pool::{ensure_pool_initialized, resolve_db_url};
pub use super::search::count_table_rows;

/// Query parameters for the Studio table viewer, supporting pagination and live search.
#[derive(Deserialize, Debug)]
pub struct TableQuery {
    pub page: Option<usize>,
    pub search: Option<String>,
}

/// Primitive SQL values that Studio can round-trip without guessing a
/// backend-specific codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StudioColumnKind {
    Text,
    Integer,
    Float,
    Boolean,
    Unsupported,
}

impl StudioColumnKind {
    pub(crate) fn from_database_type(database_type: &str) -> Self {
        let normalized = database_type.trim().to_ascii_lowercase();
        // MariaDB and MySQL 5.7 report `int(10) unsigned` (MySQL 8: `int
        // unsigned`). The signed and floating codecs cannot represent every
        // unsigned or zero-filled value, so such columns stay read-only.
        if normalized.contains("unsigned") || normalized.contains("zerofill") {
            return Self::Unsupported;
        }
        if matches!(
            normalized.as_str(),
            "text"
                | "varchar"
                | "character varying"
                | "char"
                | "character"
                | "tinytext"
                | "mediumtext"
                | "longtext"
        ) || normalized.starts_with("varchar(")
            || normalized.starts_with("char(")
        {
            Self::Text
        } else if matches!(normalized.as_str(), "bool" | "boolean" | "tinyint(1)") {
            Self::Boolean
        } else if matches!(
            normalized.as_str(),
            "smallint"
                | "integer"
                | "int"
                | "bigint"
                | "tinyint"
                | "mediumint"
                | "int2"
                | "int4"
                | "int8"
        ) || normalized.starts_with("integer(")
            || normalized.starts_with("int(")
            || normalized.starts_with("bigint(")
            || normalized.starts_with("smallint(")
            || normalized.starts_with("tinyint(")
        {
            Self::Integer
        } else if matches!(
            normalized.as_str(),
            "real" | "float" | "double" | "double precision" | "float4" | "float8"
        ) || normalized.starts_with("float(")
            || normalized.starts_with("double(")
        {
            Self::Float
        } else {
            Self::Unsupported
        }
    }

    pub(crate) const fn is_editable(self) -> bool {
        !matches!(self, Self::Unsupported)
    }

    /// Whether a key value's rendered text binds back to exactly that value.
    /// Floating-point text is rounded (SQLite renders both `0.3` and
    /// `0.1 + 0.2` as `0.3`), so it cannot address one row.
    pub(crate) const fn round_trips_as_key(self) -> bool {
        matches!(self, Self::Text | Self::Integer | Self::Boolean)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StudioColumn {
    pub(crate) name: String,
    pub(crate) kind: StudioColumnKind,
    pub(crate) primary_key: bool,
    pub(crate) nullable: bool,
}

/// Maximum number of inspected columns Studio renders or binds for one table.
const MAX_STUDIO_COLUMNS: usize = 256;

/// Ordered, database-inspected metadata for one table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StudioTableSchema {
    pub(crate) columns: Vec<StudioColumn>,
    /// False when a primary-key column was left out because its name is
    /// outside the identifier boundary or beyond the column cap. The retained
    /// key columns are then only a prefix that can match several rows.
    pub(crate) primary_key_complete: bool,
}

impl StudioTableSchema {
    /// Row mutations need the complete primary key, and every key column must
    /// use an exact text, integer or Boolean codec whose rendered text Studio
    /// can bind back unchanged.
    pub(crate) fn supports_mutations(&self) -> bool {
        let mut key_columns = self.columns.iter().filter(|column| column.primary_key);
        self.primary_key_complete
            && key_columns.clone().next().is_some()
            && key_columns.all(|column| column.kind.round_trips_as_key())
    }

    pub(crate) fn primary_key_indices(&self) -> Vec<usize> {
        self.columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| column.primary_key.then_some(index))
            .collect()
    }
}

/// Helper function to escape standard strings manually when building raw strings
pub fn escape_html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Helper to decode any SQL Column value to String
pub fn get_any_value_as_string(
    row: &<rullst_orm::RullstDatabase as sqlx::Database>::Row,
    index: usize,
) -> String {
    if let Ok(val) = row.try_get::<String, _>(index) {
        val
    } else if let Ok(val) = row.try_get::<i64, _>(index) {
        val.to_string()
    } else if let Ok(val) = row.try_get::<i32, _>(index) {
        val.to_string()
    } else if let Ok(val) = row.try_get::<f64, _>(index) {
        val.to_string()
    } else if let Ok(val) = row.try_get::<bool, _>(index) {
        val.to_string()
    } else if let Ok(Some(val)) = row.try_get::<Option<String>, _>(index) {
        val
    } else if let Ok(Some(val)) = row.try_get::<Option<i64>, _>(index) {
        val.to_string()
    } else if let Ok(Some(val)) = row.try_get::<Option<i32>, _>(index) {
        val.to_string()
    } else if let Ok(Some(val)) = row.try_get::<Option<bool>, _>(index) {
        val.to_string()
    } else {
        "NULL".to_string()
    }
}

/// Dynamic SQLite schema tables finder
pub fn build_fetch_tables_query(driver: &str) -> &'static str {
    match driver {
        "postgres" => {
            "SELECT CAST(table_name AS VARCHAR) as name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name ASC"
        }
        "mysql" => {
            "SELECT table_name as name FROM information_schema.tables WHERE table_schema = DATABASE() ORDER BY table_name ASC"
        }
        _ => {
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name ASC"
        }
    }
}

pub async fn fetch_tables() -> Result<Vec<String>, sqlx::Error> {
    let pool = ensure_pool_initialized().await?;
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");

    let query = build_fetch_tables_query(driver);

    let rows = sqlx::query(query).fetch_all(pool).await?;

    let mut tables = Vec::new();
    for row in rows {
        if let Ok(name) = row.try_get::<String, _>(0)
            && is_safe_identifier(&name)
        {
            tables.push(name);
        }
    }
    Ok(tables)
}

/// Loads ordered column metadata for a validated table. Only metadata from the
/// active database is trusted; request-provided column names never enter SQL.
/// Columns outside the identifier boundary or the column cap are omitted; if
/// any of them belongs to the primary key, the schema reports the key as
/// incomplete so that rows are never selected by a key prefix.
pub(crate) async fn fetch_table_schema(
    pool: &rullst_orm::RullstPool,
    driver: &str,
    table: &str,
) -> Result<StudioTableSchema, sqlx::Error> {
    if !is_safe_identifier(table) {
        return Err(sqlx::Error::Configuration(
            "Studio received an unsupported SQL identifier".into(),
        ));
    }

    let query = match driver {
        "postgres" => format!(
            "SELECT CAST(c.column_name AS VARCHAR) AS name, \
                    CAST(c.data_type AS VARCHAR) AS type_name, \
                    CASE WHEN c.is_nullable = 'YES' THEN 1 ELSE 0 END AS nullable, \
                    CASE WHEN EXISTS ( \
                        SELECT 1 FROM information_schema.table_constraints tc \
                        JOIN information_schema.key_column_usage kcu \
                          ON tc.constraint_catalog = kcu.constraint_catalog \
                         AND tc.constraint_schema = kcu.constraint_schema \
                         AND tc.constraint_name = kcu.constraint_name \
                        WHERE tc.constraint_type = 'PRIMARY KEY' \
                          AND tc.table_schema = c.table_schema \
                          AND tc.table_name = c.table_name \
                          AND kcu.column_name = c.column_name \
                    ) THEN 1 ELSE 0 END AS pk \
             FROM information_schema.columns c \
             WHERE c.table_name = '{table}' AND c.table_schema = 'public' \
             ORDER BY c.ordinal_position"
        ),
        "mysql" => format!(
            "SELECT column_name AS name, column_type AS type_name, \
                    CASE WHEN is_nullable = 'YES' THEN 1 ELSE 0 END AS nullable, \
                    CASE WHEN column_key = 'PRI' THEN 1 ELSE 0 END AS pk \
             FROM information_schema.columns \
             WHERE table_name = '{table}' AND table_schema = DATABASE() \
             ORDER BY ordinal_position"
        ),
        _ => format!("PRAGMA table_info(\"{table}\")"),
    };

    let rows = QueryBuilder::<rullst_orm::RullstDatabase>::new(query)
        .build()
        .fetch_all(pool)
        .await?;
    let mut columns = Vec::with_capacity(rows.len().min(MAX_STUDIO_COLUMNS));
    let mut primary_key_complete = true;
    for row in rows {
        let primary_key = row_flag(&row, "pk");
        let name = row.try_get::<String, _>("name").unwrap_or_default();
        if !is_safe_identifier(&name) || columns.len() == MAX_STUDIO_COLUMNS {
            primary_key_complete &= !primary_key;
            continue;
        }
        let database_type = if driver == "sqlite" {
            row.try_get::<String, _>("type").unwrap_or_default()
        } else {
            row.try_get::<String, _>("type_name").unwrap_or_default()
        };
        let nullable = if driver == "sqlite" {
            !row_flag(&row, "notnull") && !primary_key
        } else {
            row_flag(&row, "nullable") && !primary_key
        };
        columns.push(StudioColumn {
            name,
            kind: StudioColumnKind::from_database_type(&database_type),
            primary_key,
            nullable,
        });
    }
    Ok(StudioTableSchema {
        columns,
        primary_key_complete,
    })
}

fn row_flag(row: &<rullst_orm::RullstDatabase as sqlx::Database>::Row, column: &str) -> bool {
    row.try_get::<bool, _>(column)
        .or_else(|_| row.try_get::<i32, _>(column).map(|value| value != 0))
        .or_else(|_| row.try_get::<i64, _>(column).map(|value| value != 0))
        .unwrap_or(false)
}

/// Legacy column-name query kept for API compatibility. Studio itself uses
/// the inspected, column-capped schema from `fetch_table_schema`.
pub fn build_schema_query(driver: &str, clean_table: &str) -> String {
    match driver {
        "postgres" => format!(
            "SELECT CAST(column_name AS VARCHAR) as name FROM information_schema.columns WHERE table_name = '{}' AND table_schema = 'public'",
            clean_table
        ),
        "mysql" => format!(
            "SELECT column_name as name FROM information_schema.columns WHERE table_name = '{}' AND table_schema = DATABASE()",
            clean_table
        ),
        _ => format!("PRAGMA table_info(\"{}\")", clean_table),
    }
}

/// Helper to build table headers HTML
pub fn build_headers_html(col_names: &[String], primary_keys: &[usize]) -> String {
    col_names.iter().enumerate().fold(
        String::with_capacity(col_names.len() * 128),
        |mut acc, (i, col)| {
            let is_pk = primary_keys.contains(&i);
            let pk_badge = if is_pk {
                "<span class=\"ml-1.5 text-[9px] font-extrabold tracking-widest bg-sky-500/10 text-sky-400 border border-sky-500/20 px-1 py-0.2 rounded font-mono\">PK</span>"
            } else {
                ""
            };
            let _ = write!(
                acc,
                "<th scope=\"col\" class=\"px-6 py-3.5 text-left text-xs font-bold text-slate-400 tracking-wider uppercase border-b border-slate-800/80\">\n                <div class=\"flex items-center\">{} {}</div>\n            </th>",
                escape_html_attr(col), pk_badge
            );
            acc
        },
    )
}

/// Helper to build table rows HTML. Cell text is cut to 256 characters.
#[cfg_attr(mutants, mutants::skip)]
pub fn build_rows_html(
    records: &[<rullst_orm::RullstDatabase as sqlx::Database>::Row],
    col_names: &[String],
) -> String {
    if records.is_empty() {
        let cols_len = col_names.len().max(1);
        return format!(
            "<tr>\n                <td colspan=\"{}\" class=\"px-6 py-16 text-center text-sm text-slate-500 font-medium bg-slate-900/20\">\n                    No records found inside this table.\n                </td>\n            </tr>",
            cols_len
        );
    }

    records.iter().fold(
        String::with_capacity(records.len() * col_names.len() * 64),
        |mut rows_html, row| {
            rows_html.push_str("<tr class=\"border-b border-slate-800/40 hover:bg-slate-900/30 transition duration-150\">");
            for i in 0..col_names.len() {
                let cell_val = get_any_value_as_string(row, i);
                let is_null = cell_val == "NULL";
                let text_class = if is_null {
                    "text-slate-600 font-mono italic"
                } else {
                    "text-slate-300"
                };
                let _ = write!(
                    rows_html,
                    "<td class=\"px-6 py-4 text-sm truncate max-w-xs {}\">{}</td>",
                    text_class,
                    escape_html_attr(&display_cell(&cell_val))
                );
            }
            rows_html.push_str("</tr>");
            rows_html
        },
    )
}

/// Engine label of the pool that Studio queries. It never parses
/// `DATABASE_URL`, which may name another database than an explicitly
/// initialized pool.
pub fn resolve_driver_display_name() -> String {
    driver_display_name(rullst_core::db::safe_driver())
}

pub(crate) fn driver_display_name(driver: Option<&str>) -> String {
    match driver {
        Some("postgres") => "POSTGRESQL".to_string(),
        Some("mysql") => "MYSQL / MARIADB".to_string(),
        Some("sqlite") => "SQLITE".to_string(),
        Some(other) => other.to_ascii_uppercase(),
        None => "NOT CONNECTED".to_string(),
    }
}
