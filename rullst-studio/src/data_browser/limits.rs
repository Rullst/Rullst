//! Byte and character bounds for the Studio table view.

use super::db::quote_table_name;
use std::borrow::Cow;

/// Longest value, in bytes, that a row mutation binds. Rows whose key text is
/// longer offer no row actions.
pub(crate) const MAX_CELL_BYTES: usize = 16 * 1024;

/// Longest cell text rendered in the table view, in characters. Longer values
/// are cut and end with an ellipsis.
pub(crate) const MAX_DISPLAY_CHARS: usize = 256;

/// Longest search term, in bytes, accepted by the table view.
pub(crate) const MAX_SEARCH_BYTES: usize = 256;

/// Selects at most `max_chars` characters of a column's text form, so the
/// database never returns a complete large value to Studio.
pub(crate) fn bounded_text_expression(driver: &str, column: &str, max_chars: usize) -> String {
    let quoted = quote_table_name(driver, column);
    if driver == "mysql" {
        format!("SUBSTRING(CAST({quoted} AS CHAR), 1, {max_chars}) AS {quoted}")
    } else {
        format!("SUBSTR(CAST({quoted} AS TEXT), 1, {max_chars}) AS {quoted}")
    }
}

/// Cuts a cell's text to [`MAX_DISPLAY_CHARS`] characters for display.
pub(crate) fn display_cell(value: &str) -> Cow<'_, str> {
    match value.char_indices().nth(MAX_DISPLAY_CHARS) {
        Some((cut, _)) => Cow::Owned(format!("{}…", &value[..cut])),
        None => Cow::Borrowed(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_cells_are_cut_on_character_boundaries() {
        let exact = "é".repeat(MAX_DISPLAY_CHARS);
        assert!(matches!(display_cell(&exact), Cow::Borrowed(_)));

        let longer = format!("{exact}tail");
        let shown = display_cell(&longer);
        assert_eq!(shown.chars().count(), MAX_DISPLAY_CHARS + 1);
        assert!(shown.ends_with("é…"));
        assert!(!shown.contains("tail"));
    }

    #[test]
    fn bounded_selects_use_a_constant_length_for_each_backend() {
        assert_eq!(
            bounded_text_expression("sqlite", "body", 257),
            "SUBSTR(CAST(\"body\" AS TEXT), 1, 257) AS \"body\""
        );
        assert_eq!(
            bounded_text_expression("postgres", "body", 257),
            "SUBSTR(CAST(\"body\" AS TEXT), 1, 257) AS \"body\""
        );
        assert_eq!(
            bounded_text_expression("mysql", "body", 257),
            "SUBSTRING(CAST(`body` AS CHAR), 1, 257) AS `body`"
        );
    }
}
