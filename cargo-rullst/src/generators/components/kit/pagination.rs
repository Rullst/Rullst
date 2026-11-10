//! Pagination, copied by `cargo rullst ui add pagination`; the application
//! owns this file. Plain links, so pages work without JavaScript; add
//! `hx-boost="true"` on an element around it (or the page's `<body>`) to let
//! htmx swap the page instead of reloading it.
//!
//! ```text
//! use crate::ui::pagination::{pagination, PaginationOptions};
//!
//! pagination(&PaginationOptions { page: 3, pages: 12, ..Default::default() }, |page| {
//!     format!("/invoices?page={page}")
//! })
//! ```
//!
//! Accessibility: a `<nav>` named "Pagination"; the current page is marked
//! with `aria-current="page"`, each number link is named "Page N", and the
//! previous and next links are left out on the first and last page instead
//! of being shown disabled.

use rullst::html;
use rullst::html::Markup;

/// Options of [`pagination`]; the labels are the texts to translate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaginationOptions<'a> {
    /// The current page, from 1.
    pub page: u32,
    /// The number of pages; nothing is rendered for 1 or fewer.
    pub pages: u32,
    pub label: &'a str,
    pub previous: &'a str,
    pub next: &'a str,
    /// Text before the page number in each link's accessible name.
    pub page_label: &'a str,
}

impl Default for PaginationOptions<'_> {
    fn default() -> Self {
        Self {
            page: 1,
            pages: 1,
            label: "Pagination",
            previous: "Previous",
            next: "Next",
            page_label: "Page",
        }
    }
}

/// The pages shown for `page` of `pages`: all of them up to 7, else the
/// first, the last, the current one with its neighbours (five pages next to
/// either end) and `None` for each gap of two pages or more.
pub fn window(page: u32, pages: u32) -> Vec<Option<u32>> {
    let page = page.clamp(1, pages.max(1));
    if pages <= 7 {
        return (1..=pages).map(Some).collect();
    }
    // pages >= 8, so both ranges are non-empty.
    let start = page.saturating_sub(1).clamp(2, pages - 4);
    let end = (page + 1).clamp(5, pages - 1);
    let mut shown = vec![Some(1)];
    match start {
        3 => shown.push(Some(2)),
        4.. => shown.push(None),
        _ => {}
    }
    shown.extend((start..=end).map(Some));
    match pages - end {
        2 => shown.push(Some(pages - 1)),
        3.. => shown.push(None),
        _ => {}
    }
    shown.push(Some(pages));
    shown
}

const LINK: &str = "inline-flex h-9 min-w-9 items-center justify-center gap-1 rounded-md px-3 text-sm font-medium text-slate-700 hover:bg-slate-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-sky-600 dark:text-slate-300 dark:hover:bg-slate-800 dark:focus-visible:outline-sky-400";
const CURRENT: &str = "inline-flex h-9 min-w-9 items-center justify-center rounded-md bg-sky-700 px-3 text-sm font-semibold text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-sky-600 dark:bg-sky-500 dark:text-slate-950 dark:focus-visible:outline-sky-400";

/// Links to the pages around `options.page`; `href` builds a page's URL.
pub fn pagination(options: &PaginationOptions<'_>, href: impl Fn(u32) -> String) -> Markup {
    if options.pages <= 1 {
        return Markup::default();
    }
    let page = options.page.clamp(1, options.pages);
    html! {
        <nav aria-label={options.label}>
            <ul class="flex flex-wrap items-center gap-1">
                @if page > 1 {
                    <li><a href={href(page - 1)} rel="prev" class={LINK}><span aria-hidden="true">"←"</span>{options.previous}</a></li>
                }
                @for item in window(page, options.pages) {
                    @match item {
                        Some(number) if number == page => {
                            <li><a href={href(number)} aria-current="page" aria-label={format!("{} {number}", options.page_label)} class={CURRENT}>{number}</a></li>
                        }
                        Some(number) => {
                            <li><a href={href(number)} aria-label={format!("{} {number}", options.page_label)} class={LINK}>{number}</a></li>
                        }
                        None => {
                            <li><span class="inline-flex h-9 min-w-9 items-center justify-center text-sm text-slate-600 dark:text-slate-400" aria-hidden="true">"…"</span></li>
                        }
                    }
                }
                @if page < options.pages {
                    <li><a href={href(page + 1)} rel="next" class={LINK}>{options.next}<span aria-hidden="true">"→"</span></a></li>
                }
            </ul>
        </nav>
    }
}
