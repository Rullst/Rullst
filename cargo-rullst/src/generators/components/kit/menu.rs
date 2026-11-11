//! Dropdown menu, copied by `cargo rullst ui add menu`; the application owns
//! this file and `static/ui/menu.js`. Uses `ui::button`.
//!
//! ```text
//! use crate::ui::button::Variant;
//! use crate::ui::menu::{menu, Item, MenuOptions};
//!
//! html! {
//!     {menu(&MenuOptions { id: "account", label: "Account", ..Default::default() }, &[
//!         Item::Link { label: "Profile", href: "/profile" },
//!         Item::Separator,
//!         Item::Submit { label: "Sign out", form: "logout" },
//!     ])}
//!     <form id="logout" method="post" action="/logout">…</form>
//! }
//! ```
//!
//! The list is a `popover`: the button opens and closes it without script
//! (`popovertarget`, in every current browser), and the browser closes it on
//! Escape or a click elsewhere. `static/ui/menu.js` (a same-origin module)
//! places it under the button and adds the keyboard of the ARIA menu button
//! pattern: Enter, Space, ArrowDown or ArrowUp on the button opens it and
//! focuses the first (ArrowUp: last) item; ArrowDown, ArrowUp, Home and End
//! move between items; Escape closes it and returns focus to the button; Tab
//! closes it and moves on. It keeps `aria-expanded` on the button in step.
//!
//! Items are links (navigation) or submit buttons tied to a form elsewhere on
//! the page by its `id` (actions, which need the form's CSRF token).

use super::button::{self, Size, Variant};
use rullst::html;
use rullst::html::Markup;

/// One entry of a [`menu`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item<'a> {
    /// Navigates to `href`.
    Link { label: &'a str, href: &'a str },
    /// Submits the form whose `id` is `form`.
    Submit { label: &'a str, form: &'a str },
    /// A line between groups of items.
    Separator,
}

/// Options of [`menu`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuOptions<'a> {
    /// The menu's `id`, unique on the page; the button is `<id>-button`.
    pub id: &'a str,
    /// The button's text, which also names the menu.
    pub label: &'a str,
    pub variant: Variant,
}

impl Default for MenuOptions<'_> {
    fn default() -> Self {
        Self {
            id: "menu",
            label: "Menu",
            variant: Variant::Outline,
        }
    }
}

const ITEM: &str = "flex w-full cursor-pointer items-center rounded-md px-3 py-2 text-left text-sm text-slate-900 hover:bg-slate-100 focus-visible:bg-slate-100 focus-visible:outline-2 focus-visible:outline-sky-600 dark:text-slate-100 dark:hover:bg-slate-800 dark:focus-visible:bg-slate-800 dark:focus-visible:outline-sky-400";

/// The same-origin module that adds the keyboard and the placement.
pub fn script() -> Markup {
    html! {
        <script type="module" src={rullst::assets::url("ui/menu.js")}></script>
    }
}

fn item(entry: &Item<'_>) -> Markup {
    match *entry {
        Item::Link { label, href } => html! {
            <a role="menuitem" tabindex="-1" href={href} class={ITEM}>{label}</a>
        },
        Item::Submit { label, form } => html! {
            <button role="menuitem" tabindex="-1" type="submit" form={form} class={ITEM}>{label}</button>
        },
        Item::Separator => html! {
            <div role="separator" class="my-1 h-px bg-slate-200 dark:bg-slate-700"></div>
        },
    }
}

/// The menu button and its closed list of `items`, with its script.
pub fn menu(options: &MenuOptions<'_>, items: &[Item<'_>]) -> Markup {
    let button_id = format!("{}-button", options.id);
    html! {
        <button type="button" id={button_id.as_str()} popovertarget={options.id} aria-haspopup="menu" aria-controls={options.id} aria-expanded="false" class={button::classes(options.variant, Size::Md)}>
            {options.label}
            <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="m6 9 6 6 6-6"></path>
            </svg>
        </button>
        <div id={options.id} popover role="menu" aria-labelledby={button_id.as_str()} class="m-0 min-w-48 rounded-lg border border-slate-200 bg-white p-1 text-slate-900 shadow-lg dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100">
            @for entry in items {
                {item(entry)}
            }
        </div>
        {script()}
    }
}
