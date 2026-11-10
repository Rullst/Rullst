//! Button, copied by `cargo rullst ui add button`; the application owns this
//! file. Variants and sizes are Tailwind utility classes written in full, so
//! the Tailwind CLI finds every one of them in this file.
//!
//! ```text
//! use crate::ui::button::{self, ButtonOptions, Kind, Variant};
//!
//! button::button("Save", &ButtonOptions { kind: Kind::Submit, ..Default::default() })
//!
//! // Any other attribute (hx-post, name, form): write the element yourself.
//! <button type="button" hx-post="/archive" class={button::classes(Variant::Outline, button::Size::Sm)}>
//!     {button::spinner()} "Archive"
//! </button>
//! ```
//!
//! Loading state: with `loading: true` the button holds a spinner that shows
//! while htmx marks the button, or an element around it, with its
//! `htmx-request` class. Point the form's `hx-indicator` at the button and
//! disable it during the request with `hx-disabled-elt`:
//! `<form hx-post="/save" hx-indicator="#save" hx-disabled-elt="#save">`.

use rullst::html;
use rullst::html::Markup;

/// The look of a button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// The main action of a page or form.
    #[default]
    Primary,
    /// A second action next to a primary one.
    Secondary,
    /// A bordered button for less important actions.
    Outline,
    /// No background until hovered, for toolbars and menus.
    Ghost,
    /// An action that deletes or cannot be undone.
    Destructive,
    /// Looks like a link and acts like a button.
    Link,
}

/// The size of a button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
    /// A square button holding one icon; give it an `aria-label`.
    Icon,
}

/// The `type` of the button element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    /// `type="button"`: does nothing by itself (htmx, dialogs, menus).
    #[default]
    Button,
    /// `type="submit"`: submits its form.
    Submit,
    /// `type="reset"`: resets its form.
    Reset,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Button => "button",
            Kind::Submit => "submit",
            Kind::Reset => "reset",
        }
    }
}

/// Options of [`button`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ButtonOptions {
    pub variant: Variant,
    pub size: Size,
    pub kind: Kind,
    /// The element's `id`, for `hx-indicator="#id"` or a `<label for>`.
    pub id: Option<String>,
    /// Writes the `disabled` attribute.
    pub disabled: bool,
    /// Adds the spinner shown while an htmx request is in flight.
    pub loading: bool,
}

const BASE: &str = "inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 rounded-md font-medium whitespace-nowrap transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-sky-600 disabled:cursor-not-allowed disabled:opacity-60 dark:focus-visible:outline-sky-400 [.htmx-request_&]:cursor-progress";

fn variant_classes(variant: Variant) -> &'static str {
    match variant {
        Variant::Primary => {
            "bg-sky-700 text-white hover:bg-sky-800 dark:bg-sky-500 dark:text-slate-950 dark:hover:bg-sky-400"
        }
        Variant::Secondary => {
            "bg-slate-100 text-slate-900 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-100 dark:hover:bg-slate-700"
        }
        Variant::Outline => {
            "border border-slate-300 bg-white text-slate-900 hover:bg-slate-100 dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100 dark:hover:bg-slate-800"
        }
        Variant::Ghost => {
            "text-slate-900 hover:bg-slate-100 dark:text-slate-100 dark:hover:bg-slate-800"
        }
        Variant::Destructive => {
            "bg-red-700 text-white hover:bg-red-800 dark:bg-red-500 dark:text-slate-950 dark:hover:bg-red-400"
        }
        Variant::Link => "text-sky-700 underline-offset-4 hover:underline dark:text-sky-400",
    }
}

fn size_classes(size: Size) -> &'static str {
    match size {
        Size::Sm => "h-8 px-3 text-sm",
        Size::Md => "h-10 px-4 text-sm",
        Size::Lg => "h-12 px-6 text-base",
        Size::Icon => "size-10",
    }
}

/// The classes of a button with `variant` and `size`, for a `<button>` or an
/// `<a>` written by hand.
pub fn classes(variant: Variant, size: Size) -> String {
    format!("{BASE} {} {}", variant_classes(variant), size_classes(size))
}

/// The spinner shown while htmx marks the button, or an element around it,
/// with `htmx-request`. It also has htmx's `htmx-indicator` class, so pages
/// rendered with `render_page*` hide it without Tailwind.
pub fn spinner() -> Markup {
    html! {
        <svg class="htmx-indicator hidden size-4 animate-spin [.htmx-request_&]:inline-block" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 0 1 8-8v4a4 4 0 0 0-4 4H4z"></path>
        </svg>
    }
}

/// A `<button>` labelled `label`.
pub fn button(label: &str, options: &ButtonOptions) -> Markup {
    let indicator = options.loading.then(spinner);
    html! {
        <button type={options.kind.as_str()} id?={options.id.as_deref()} disabled?={options.disabled} class={classes(options.variant, options.size)}>
            {indicator}
            <span>{label}</span>
        </button>
    }
}

/// An `<a>` that looks like a button: navigation, not an action.
pub fn link(label: &str, href: &str, variant: Variant, size: Size) -> Markup {
    html! {
        <a href={href} class={classes(variant, size)}>{label}</a>
    }
}
