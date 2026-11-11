//! Modal dialog, copied by `cargo rullst ui add dialog`; the application
//! owns this file and `static/ui/dialog.js`. Uses `ui::button`.
//!
//! ```text
//! use crate::ui::button::Variant;
//! use crate::ui::dialog::{self, DialogOptions};
//!
//! html! {
//!     {dialog::trigger("delete-project", "Delete project", Variant::Destructive)}
//!     {dialog::dialog(
//!         &DialogOptions { id: "delete-project", title: "Delete the project?", ..Default::default() },
//!         html! { <p>"Its invoices are kept for five years."</p> },
//!         Some(html! {
//!             {dialog::close("delete-project", "Cancel", Variant::Outline)}
//!             <form method="post" action="/projects/7/delete">…</form>
//!         }),
//!     )}
//! }
//! ```
//!
//! A native `<dialog>` opened with `showModal()`: the browser makes the rest
//! of the page inert, keeps focus inside, closes it on Escape and returns
//! focus to the button that opened it. The buttons use the HTML invoker
//! commands (`commandfor` and `command="show-modal"` / `"close"`), built into
//! Chrome 135, Firefox 144 and Safari 26.2 and later; `static/ui/dialog.js`
//! (a same-origin module, no inline script) does the same in older browsers
//! and returns focus where a browser does not.
//!
//! Accessibility: the dialog is named by its title (`aria-labelledby`) and
//! described by `description` when given; focus starts on its first control,
//! the close button in the corner, or on an element marked `autofocus`.

use super::button::{self, Size, Variant};
use rullst::html;
use rullst::html::Markup;

/// The width of a dialog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Width {
    Sm,
    #[default]
    Md,
    Lg,
}

/// Options of [`dialog`]; `close_label` is the text to translate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialogOptions<'a> {
    /// The dialog's `id`, unique on the page; triggers refer to it.
    pub id: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub width: Width,
    /// The accessible name of the corner close button.
    pub close_label: &'a str,
}

impl Default for DialogOptions<'_> {
    fn default() -> Self {
        Self {
            id: "dialog",
            title: "",
            description: None,
            width: Width::Md,
            close_label: "Close",
        }
    }
}

fn width_classes(width: Width) -> &'static str {
    match width {
        Width::Sm => "max-w-sm",
        Width::Md => "max-w-lg",
        Width::Lg => "max-w-2xl",
    }
}

/// The same-origin module that polyfills the commands and returns focus.
pub fn script() -> Markup {
    html! {
        <script type="module" src={rullst::assets::url("ui/dialog.js")}></script>
    }
}

/// A button that opens the dialog `id`.
pub fn trigger(id: &str, label: &str, variant: Variant) -> Markup {
    html! {
        <button type="button" commandfor={id} command="show-modal" aria-haspopup="dialog" class={button::classes(variant, Size::Md)}>{label}</button>
    }
}

/// A button that closes the dialog `id`, for its footer.
pub fn close(id: &str, label: &str, variant: Variant) -> Markup {
    html! {
        <button type="button" commandfor={id} command="close" class={button::classes(variant, Size::Md)}>{label}</button>
    }
}

/// The dialog holding `body`, with `footer` (usually [`close`] and the
/// confirming action) below it. It renders closed, with its script.
pub fn dialog(options: &DialogOptions<'_>, body: Markup, footer: Option<Markup>) -> Markup {
    let title_id = format!("{}-title", options.id);
    let description_id = options
        .description
        .map(|_| format!("{}-description", options.id));
    let classes = format!(
        "m-auto w-[calc(100%-2rem)] {} rounded-xl border border-slate-200 bg-white p-0 text-slate-900 shadow-xl backdrop:bg-slate-950/60 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100",
        width_classes(options.width)
    );
    html! {
        <dialog id={options.id} aria-labelledby={title_id.as_str()} aria-describedby?={description_id.as_deref()} class={classes}>
            <div class="flex items-start justify-between gap-4 px-6 pt-6">
                <h2 id={title_id.as_str()} class="text-lg leading-tight font-semibold">{options.title}</h2>
                <button type="button" commandfor={options.id} command="close" aria-label={options.close_label} class="-m-1 inline-flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-md text-slate-600 hover:bg-slate-100 hover:text-slate-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-sky-600 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100 dark:focus-visible:outline-sky-400">
                    <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
                        <path d="M18 6 6 18M6 6l12 12"></path>
                    </svg>
                </button>
            </div>
            @if let (Some(description), Some(id)) = (options.description, description_id.as_deref()) {
                <p id={id} class="px-6 pt-2 text-sm text-slate-600 dark:text-slate-400">{description}</p>
            }
            <div class="px-6 py-4 text-sm">{body}</div>
            @if let Some(footer) = footer {
                <div class="flex flex-wrap items-center justify-end gap-2 border-t border-slate-200 px-6 py-4 dark:border-slate-800">{footer}</div>
            }
        </dialog>
        {script()}
    }
}
