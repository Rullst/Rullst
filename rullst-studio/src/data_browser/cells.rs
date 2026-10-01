//! Decoding and rendering of Studio table cells.
//!
//! SQL NULL, a value Studio can show and a present value it cannot decode
//! (for example a BLOB that is not UTF-8) are kept apart, so a column that
//! holds data is never reported as NULL.

use super::db::escape_html_attr;
use super::limits::display_cell;
use sqlx::{Row, ValueRef};

type StudioRow = <rullst_orm::RullstDatabase as sqlx::Database>::Row;

/// Text shown for a present value that no supported codec decodes.
pub(crate) const UNREADABLE_MARKER: &str = "unreadable";

/// Shared classes of every table cell.
const CELL_CLASSES: &str = "px-6 py-4 text-sm truncate max-w-xs";

/// One table cell as Studio shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StudioCell {
    /// The column is SQL NULL.
    Null,
    /// A present value decoded as text, an integer, a float or a Boolean.
    Value(String),
    /// A present value no supported codec decodes.
    Unreadable,
}

impl StudioCell {
    /// The cell as plain text: `NULL`, the value or the unreadable marker.
    pub(crate) fn into_text(self) -> String {
        match self {
            Self::Null => "NULL".to_string(),
            Self::Value(value) => value,
            Self::Unreadable => UNREADABLE_MARKER.to_string(),
        }
    }
}

/// Decodes column `index` of `row`, checking for SQL NULL before any codec.
pub(crate) fn decode_cell(row: &StudioRow, index: usize) -> StudioCell {
    match row.try_get_raw(index) {
        Ok(raw) if raw.is_null() => return StudioCell::Null,
        Ok(_) => {}
        Err(_) => return StudioCell::Unreadable,
    }
    row.try_get::<String, _>(index)
        .or_else(|_| row.try_get::<i64, _>(index).map(|value| value.to_string()))
        .or_else(|_| row.try_get::<i32, _>(index).map(|value| value.to_string()))
        .or_else(|_| row.try_get::<f64, _>(index).map(|value| value.to_string()))
        .or_else(|_| row.try_get::<bool, _>(index).map(|value| value.to_string()))
        .map_or(StudioCell::Unreadable, StudioCell::Value)
}

/// Renders one `<td>`. NULL and unreadable cells use distinct muted markers,
/// so neither can be mistaken for a stored text such as `NULL`.
pub(crate) fn cell_html(cell: &StudioCell) -> String {
    match cell {
        StudioCell::Null => {
            format!("<td class=\"{CELL_CLASSES} text-slate-600 font-mono italic\">NULL</td>")
        }
        StudioCell::Unreadable => format!(
            "<td class=\"{CELL_CLASSES} text-amber-400 font-mono italic\" \
             title=\"The stored value is not text, a number or a Boolean\">{UNREADABLE_MARKER}</td>"
        ),
        StudioCell::Value(value) => format!(
            "<td class=\"{CELL_CLASSES} text-slate-300\">{}</td>",
            escape_html_attr(&display_cell(value))
        ),
    }
}

#[cfg(test)]
#[cfg(not(miri))]
#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn present_undecodable_values_are_not_reported_as_null() {
        let pool = super::super::pool::test_sqlite_pool().await;
        let row = sqlx::query(
            "SELECT x'ff00fe' AS digest, NULL AS missing, 'NULL' AS literal, 7 AS number",
        )
        .fetch_one(pool)
        .await
        .expect("cell fixture row");

        assert_eq!(decode_cell(&row, 0), StudioCell::Unreadable);
        assert_eq!(decode_cell(&row, 1), StudioCell::Null);
        assert_eq!(decode_cell(&row, 2), StudioCell::Value("NULL".to_string()));
        assert_eq!(decode_cell(&row, 3), StudioCell::Value("7".to_string()));

        let unreadable = cell_html(&decode_cell(&row, 0));
        assert!(unreadable.contains(">unreadable</td>"), "{unreadable}");
        assert!(!unreadable.contains("text-slate-600"), "{unreadable}");
        let null = cell_html(&decode_cell(&row, 1));
        assert!(null.contains("text-slate-600 font-mono italic\">NULL</td>"));
        // A stored text `NULL` is a value, not the NULL marker.
        let literal = cell_html(&decode_cell(&row, 2));
        assert!(literal.contains("text-slate-300\">NULL</td>"), "{literal}");
    }
}
