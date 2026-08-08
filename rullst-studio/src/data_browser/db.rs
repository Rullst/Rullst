//! Studio Database Inspection & Query Helpers

use serde::Deserialize;
use sqlx::{QueryBuilder, Row};
use std::fmt::Write;

/// Query parameters for the Studio table viewer, supporting pagination and live search.
#[derive(Deserialize, Debug)]
pub struct TableQuery {
    pub page: Option<usize>,
    pub search: Option<String>,
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

pub fn resolve_db_url(provided: &str) -> String {
    if !provided.trim().is_empty() {
        return provided.trim().to_string();
    }
    if let Ok(env_url) = std::env::var("DATABASE_URL")
        && !env_url.trim().is_empty()
    {
        return env_url.trim().to_string();
    }
    if let Ok(toml_content) = std::fs::read_to_string("Rullst.toml") {
        for line in toml_content.lines() {
            let trimmed = line.trim();
            if (trimmed.starts_with("url =") || trimmed.starts_with("url="))
                && let Some(val) = trimmed.split('=').nth(1)
            {
                let clean = val.trim().trim_matches('"').trim_matches('\'');
                if !clean.is_empty() {
                    return clean.to_string();
                }
            }
        }
    }
    if std::path::Path::new("db.sqlite").exists() {
        return "sqlite://db.sqlite".to_string();
    }
    if std::path::Path::new("rullst.db").exists() {
        return "sqlite://rullst.db".to_string();
    }
    "sqlite://db.sqlite".to_string()
}

pub async fn ensure_pool_initialized() -> Result<&'static rullst_core::db::RullstPool, sqlx::Error>
{
    if let Some(pool) = rullst_core::db::safe_pool() {
        Ok(pool)
    } else {
        let db_url = resolve_db_url("");
        let _ = rullst_orm::Orm::init(&db_url).await;
        rullst_core::db::safe_pool()
            .ok_or_else(|| sqlx::Error::Configuration("Database pool not initialized".into()))
    }
}

pub async fn fetch_tables() -> Result<Vec<String>, sqlx::Error> {
    let pool = ensure_pool_initialized().await?;
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");

    let query = build_fetch_tables_query(driver);

    let rows = sqlx::query(query).fetch_all(pool).await?;

    let mut tables = Vec::new();
    for row in rows {
        if let Ok(name) = row.try_get::<String, _>(0) {
            tables.push(name);
        }
    }
    Ok(tables)
}

pub fn quote_table_name(driver: &str, clean_table: &str) -> String {
    if driver == "mysql" {
        format!("`{}`", clean_table)
    } else {
        format!("\"{}\"", clean_table)
    }
}

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

/// Dynamic SQLite table row counter
pub async fn count_table_rows(
    table: &str,
    search_query: Option<&str>,
) -> Result<usize, sqlx::Error> {
    let pool = ensure_pool_initialized().await?;
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let clean_table = sanitize_identifier(table);

    let quoted_table = quote_table_name(driver, &clean_table);

    let mut qb: QueryBuilder<rullst_orm::RullstDatabase> =
        QueryBuilder::new(format!("SELECT COUNT(*) FROM {}", quoted_table));

    if let Some(search) = search_query
        && !search.is_empty()
    {
        let schema_query = build_schema_query(driver, &clean_table);
        if let Ok(columns_rows) = QueryBuilder::<rullst_orm::RullstDatabase>::new(schema_query)
            .build()
            .fetch_all(pool)
            .await
        {
            let mut col_names = Vec::new();
            for r in columns_rows {
                if let Ok(name) = r.try_get::<String, _>("name") {
                    col_names.push(name);
                }
            }
            if !col_names.is_empty() {
                qb.push(" WHERE ");
                let mut separated = qb.separated(" OR ");
                for col in &col_names {
                    separated.push(build_search_clause(driver, col));
                    separated.push_bind_unseparated(format!("%{}%", search));
                }
            }
        }
    }

    let row = qb.build().fetch_one(pool).await?;
    let count: i64 = row.try_get(0).unwrap_or(0);
    Ok(count as usize)
}

/// Sanitize table and column names to prevent SQL injections in dynamic queries
pub fn sanitize_identifier(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(64) // Strict length limit for security
        .collect()
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

/// Helper to build table rows HTML
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
                    escape_html_attr(&cell_val)
                );
            }
            rows_html.push_str("</tr>");
            rows_html
        },
    )
}

pub fn resolve_driver_display_name() -> String {
    if let Ok(env_str) = std::fs::read_to_string(".env") {
        for line in env_str.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("DATABASE_URL=") {
                let url = trimmed
                    .trim_start_matches("DATABASE_URL=")
                    .trim_matches('"')
                    .trim_matches('\'');
                if url.contains("turso") || url.starts_with("libsql") {
                    return "TURSO / LIBSQL".to_string();
                } else if url.starts_with("postgres") || url.starts_with("postgresql") {
                    return "POSTGRESQL".to_string();
                } else if url.starts_with("mysql") || url.starts_with("mariadb") {
                    return "MYSQL / MARIADB".to_string();
                } else if url.starts_with("sqlite") {
                    return "SQLITE".to_string();
                }
            }
        }
    }
    rullst_core::db::safe_driver()
        .unwrap_or("sqlite")
        .to_uppercase()
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_sanitize_identifier_length_bound() {
        let id: [u8; 8] = kani::any();
        if let Ok(s) = std::str::from_utf8(&id) {
            let clean = sanitize_identifier(s);
            assert!(clean.len() <= 64);
        }
    }
}

