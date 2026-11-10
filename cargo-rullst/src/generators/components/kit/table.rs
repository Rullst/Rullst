//! Data table, copied by `cargo rullst ui add table`; the application owns
//! this file. A real `<table>` inside a scroll container, with an empty state.
//!
//! ```text
//! use crate::ui::table::{table, Align, Column, TableOptions};
//!
//! let rows = invoices.iter().map(|invoice| vec![
//!     html! { {invoice.number.as_str()} },
//!     html! { {invoice.total.to_string()} },
//! ]).collect();
//! table(
//!     &TableOptions { id: "invoices", caption: "Invoices", ..Default::default() },
//!     &[Column::new("Number"), Column { label: "Total", align: Align::End }],
//!     rows,
//! )
//! ```
//!
//! Accessibility: the caption names the table (shown, or kept for screen
//! readers with `show_caption: false`), header cells have `scope="col"`, and
//! the scroll container is a named, keyboard-focusable region, so a table
//! wider than the screen can be scrolled with the arrow keys.

use rullst::html;
use rullst::html::Markup;

/// The alignment of a column; numbers read best aligned to the end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    End,
}

/// A column header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Column<'a> {
    pub label: &'a str,
    pub align: Align,
}

impl<'a> Column<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            align: Align::Start,
        }
    }
}

/// Options of [`table`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableOptions<'a> {
    /// Prefix of the ids (`<id>-caption`), unique on the page.
    pub id: &'a str,
    pub caption: &'a str,
    /// Shows the caption above the table; when false it is read by screen
    /// readers only.
    pub show_caption: bool,
    /// The text of the single row shown when there are no rows.
    pub empty: &'a str,
}

impl Default for TableOptions<'_> {
    fn default() -> Self {
        Self {
            id: "table",
            caption: "",
            show_caption: true,
            empty: "Nothing to show yet.",
        }
    }
}

fn align_classes(align: Align) -> &'static str {
    match align {
        Align::Start => "text-left",
        Align::End => "text-right tabular-nums",
    }
}

/// A table of `rows`, each holding one cell per column.
pub fn table(options: &TableOptions<'_>, columns: &[Column<'_>], rows: Vec<Vec<Markup>>) -> Markup {
    let caption_id = format!("{}-caption", options.id);
    let caption_classes = if options.show_caption {
        "px-4 py-3 text-left text-sm font-semibold text-slate-900 dark:text-slate-100"
    } else {
        "sr-only"
    };
    let header_cell = "px-4 py-3 font-semibold";
    html! {
        <div role="region" aria-labelledby={caption_id.as_str()} tabindex="0" class="w-full overflow-x-auto rounded-lg border border-slate-200 bg-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-sky-600 dark:border-slate-800 dark:bg-slate-900 dark:focus-visible:outline-sky-400">
            <table class="w-full border-collapse text-sm text-slate-700 dark:text-slate-300">
                <caption id={caption_id.as_str()} class={caption_classes}>{options.caption}</caption>
                <thead class="bg-slate-50 text-xs text-slate-700 uppercase dark:bg-slate-800 dark:text-slate-300">
                    <tr>
                        @for column in columns {
                            <th scope="col" class={format!("{header_cell} {}", align_classes(column.align))}>{column.label}</th>
                        }
                    </tr>
                </thead>
                <tbody>
                    @for row in &rows {
                        <tr class="border-t border-slate-200 hover:bg-slate-50 dark:border-slate-800 dark:hover:bg-slate-800/60">
                            @for (index, cell) in row.iter().enumerate() {
                                <td class={format!("px-4 py-3 {}", align_classes(columns.get(index).map_or(Align::Start, |column| column.align)))}>{cell}</td>
                            }
                        </tr>
                    }
                    @if rows.is_empty() {
                        <tr class="border-t border-slate-200 dark:border-slate-800">
                            <td colspan={columns.len().max(1)} class="px-4 py-10 text-center text-slate-600 dark:text-slate-400">{options.empty}</td>
                        </tr>
                    }
                </tbody>
            </table>
        </div>
    }
}
