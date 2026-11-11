//! Toast notifications, copied by `cargo rullst ui add toast`; the
//! application owns this file and `static/ui/toast.js`.
//!
//! Render [`region`] once per page, in the layout (for example as the last
//! element of `<body>`). A handler then shows a toast by sending htmx's
//! `HX-Trigger` header with [`trigger`]:
//!
//! ```text
//! use crate::ui::toast::{self, Tone};
//! use rullst::htmx::HtmxResponse;
//!
//! async fn save() -> HtmxResponse {
//!     HtmxResponse::new(fragment.into_string()).trigger(toast::trigger("Saved", Tone::Success))
//! }
//! ```
//!
//! htmx fires the `ui:toast` event when the response arrives and
//! `static/ui/toast.js` (a same-origin module) clones the matching
//! `<template>` of the region, sets the message as text (never as HTML) and
//! adds it to the `aria-live="polite"` list, so screen readers announce it.
//! Info and success toasts leave after six seconds, unless the pointer or the
//! focus is on them; warnings and errors stay until dismissed. Any script can
//! show one: `document.dispatchEvent(new CustomEvent("ui:toast", { detail:
//! { message: "Saved", tone: "success" } }))`. A toast to show on the next
//! full page (after a redirect) can be rendered into the region with
//! `initial`.

use rullst::html;
use rullst::html::Markup;

/// The kind of a toast.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl Tone {
    pub const ALL: [Tone; 4] = [Tone::Info, Tone::Success, Tone::Warning, Tone::Error];

    /// The name `toast.js` and the `HX-Trigger` value use.
    pub fn name(self) -> &'static str {
        match self {
            Tone::Info => "info",
            Tone::Success => "success",
            Tone::Warning => "warning",
            Tone::Error => "error",
        }
    }
}

fn tone_classes(tone: Tone) -> &'static str {
    match tone {
        Tone::Info => {
            "border-slate-200 bg-white text-slate-900 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100"
        }
        Tone::Success => {
            "border-emerald-300 bg-emerald-50 text-emerald-900 dark:border-emerald-800 dark:bg-emerald-950 dark:text-emerald-100"
        }
        Tone::Warning => {
            "border-amber-300 bg-amber-50 text-amber-950 dark:border-amber-800 dark:bg-amber-950 dark:text-amber-100"
        }
        Tone::Error => {
            "border-red-300 bg-red-50 text-red-900 dark:border-red-800 dark:bg-red-950 dark:text-red-100"
        }
    }
}

/// The value of an `HX-Trigger` header that shows `message` as a toast:
/// `{"ui:toast":{"message":"…","tone":"success"}}`. Characters outside
/// ASCII are written as `\u` escapes, so the header stays valid.
pub fn trigger(message: &str, tone: Tone) -> String {
    format!(
        "{{\"ui:toast\":{{\"message\":{},\"tone\":\"{}\"}}}}",
        json_string(message),
        tone.name()
    )
}

/// `text` as a JSON string literal of ASCII characters only.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ' '..='~' => out.push(character),
            _ => {
                let mut units = [0_u16; 2];
                for unit in character.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
    out
}

/// One toast; `dismiss` names its close button.
fn item(message: &str, tone: Tone, dismiss: &str) -> Markup {
    let classes = format!(
        "pointer-events-auto flex w-full items-start gap-3 rounded-lg border p-4 text-sm shadow-lg {}",
        tone_classes(tone)
    );
    html! {
        <div data-ui-toast data-tone={tone.name()} class={classes}>
            <p data-ui-toast-message class="flex-1">{message}</p>
            <button type="button" data-ui-toast-close aria-label={dismiss} class="-m-1 inline-flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-md opacity-80 hover:opacity-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-sky-600 dark:focus-visible:outline-sky-400">
                <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
                    <path d="M18 6 6 18M6 6l12 12"></path>
                </svg>
            </button>
        </div>
    }
}

/// Options of [`region`]; the labels are the texts to translate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegionOptions<'a> {
    /// Names the region landmark.
    pub label: &'a str,
    /// Names each toast's close button.
    pub dismiss: &'a str,
}

impl Default for RegionOptions<'_> {
    fn default() -> Self {
        Self {
            label: "Notifications",
            dismiss: "Dismiss",
        }
    }
}

/// The notification region, with toasts to show at once (`initial`), the
/// templates `toast.js` clones and its script. Render it once per page.
pub fn region(options: &RegionOptions<'_>, initial: &[(&str, Tone)]) -> Markup {
    html! {
        <section aria-label={options.label} class="pointer-events-none fixed inset-x-4 bottom-4 z-50 sm:inset-x-auto sm:right-4 sm:w-96">
            <div data-ui-toasts aria-live="polite" aria-relevant="additions" class="flex flex-col gap-2">
                @for (message, tone) in initial {
                    {item(message, *tone, options.dismiss)}
                }
            </div>
            @for tone in Tone::ALL {
                <template data-ui-toast-template={tone.name()}>{item("", tone, options.dismiss)}</template>
            }
        </section>
        <script type="module" src={rullst::assets::url("ui/toast.js")}></script>
    }
}
