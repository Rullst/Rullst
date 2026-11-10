//! Tabs, copied by `cargo rullst ui add tabs`; the application owns this
//! file and `static/ui/tabs.js`.
//!
//! ```text
//! use crate::ui::tabs::{tabs, Tab, TabsOptions};
//!
//! tabs(&TabsOptions { id: "settings", label: "Settings", selected: 0 }, vec![
//!     Tab { key: "profile", label: "Profile", panel: html! { <p>"…"</p> } },
//!     Tab { key: "billing", label: "Billing", panel: html! { <div hx-get="/billing" hx-trigger="load"></div> } },
//! ])
//! ```
//!
//! The ARIA tabs pattern, rendered on the server with the selected tab, so
//! the first paint is right. `static/ui/tabs.js` (a same-origin module)
//! switches panels: a click, or ArrowLeft/ArrowRight (wrapping), Home and End
//! on the focused tab, which selects as it moves. Only the selected tab is in
//! the Tab order; Tab then moves into its panel. Without JavaScript only the
//! selected panel shows: for content that must work without it, link each
//! tab to a URL such as `?tab=billing` and render the selected one.

use rullst::html;
use rullst::html::Markup;

/// One tab and its panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab<'a> {
    /// Unique among the tabs; part of the ids (`<id>-tab-<key>`).
    pub key: &'a str,
    pub label: &'a str,
    pub panel: Markup,
}

/// Options of [`tabs`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabsOptions<'a> {
    /// Prefix of the ids, unique on the page.
    pub id: &'a str,
    /// Names the tab list for screen readers.
    pub label: &'a str,
    /// Index of the tab selected on the first paint.
    pub selected: usize,
}

impl Default for TabsOptions<'_> {
    fn default() -> Self {
        Self {
            id: "tabs",
            label: "Sections",
            selected: 0,
        }
    }
}

const TAB: &str = "-mb-px cursor-pointer border-b-2 border-transparent px-4 py-2 text-sm font-medium text-slate-600 hover:border-slate-300 hover:text-slate-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-sky-600 aria-selected:border-sky-700 aria-selected:text-sky-800 dark:text-slate-400 dark:hover:border-slate-600 dark:hover:text-slate-100 dark:focus-visible:outline-sky-400 dark:aria-selected:border-sky-400 dark:aria-selected:text-sky-300";

/// The same-origin module that switches tabs.
pub fn script() -> Markup {
    html! {
        <script type="module" src={rullst::assets::url("ui/tabs.js")}></script>
    }
}

/// The tab list and one panel per tab, with its script.
pub fn tabs(options: &TabsOptions<'_>, tabs: Vec<Tab<'_>>) -> Markup {
    let selected = options.selected.min(tabs.len().saturating_sub(1));
    let tab_id = |key: &str| format!("{}-tab-{key}", options.id);
    let panel_id = |key: &str| format!("{}-panel-{key}", options.id);
    html! {
        <div class="space-y-4">
            <div role="tablist" aria-label={options.label} class="flex gap-1 overflow-x-auto border-b border-slate-200 dark:border-slate-800">
                @for (index, tab) in tabs.iter().enumerate() {
                    <button type="button" role="tab" id={tab_id(tab.key)} aria-controls={panel_id(tab.key)} aria-selected={index == selected} tabindex={if index == selected { "0" } else { "-1" }} class={TAB}>{tab.label}</button>
                }
            </div>
            @for (index, tab) in tabs.iter().enumerate() {
                <div role="tabpanel" id={panel_id(tab.key)} aria-labelledby={tab_id(tab.key)} tabindex="0" hidden?={index != selected} class="rounded-md text-sm text-slate-900 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-sky-600 dark:text-slate-100 dark:focus-visible:outline-sky-400">{&tab.panel}</div>
            }
        </div>
        {script()}
    }
}
