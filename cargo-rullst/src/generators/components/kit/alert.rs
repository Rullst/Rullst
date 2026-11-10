//! Alert, copied by `cargo rullst ui add alert`; the application owns this
//! file. A message box in the page's flow: a form's error summary, a notice
//! after a redirect, a warning above a setting.
//!
//! ```text
//! use crate::ui::alert::{alert, AlertOptions, Tone};
//!
//! alert(
//!     &AlertOptions { tone: Tone::Danger, title: Some("The order was not saved") },
//!     html! { <p>"Check the highlighted fields."</p> },
//! )
//! ```
//!
//! Accessibility: `Danger` and `Warning` alerts have `role="alert"`, so a
//! screen reader announces them when they appear (also when htmx swaps them
//! in); the others have `role="status"`, announced politely. The icon is
//! decorative: the title and text carry the meaning.

use rullst::html;
use rullst::html::Markup;

/// The kind of message.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Info,
    Success,
    Warning,
    Danger,
}

/// Options of [`alert`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlertOptions<'a> {
    pub tone: Tone,
    /// A bold first line.
    pub title: Option<&'a str>,
}

fn tone_classes(tone: Tone) -> &'static str {
    match tone {
        Tone::Info => {
            "border-sky-300 bg-sky-50 text-sky-900 dark:border-sky-800 dark:bg-sky-950 dark:text-sky-100"
        }
        Tone::Success => {
            "border-emerald-300 bg-emerald-50 text-emerald-900 dark:border-emerald-800 dark:bg-emerald-950 dark:text-emerald-100"
        }
        Tone::Warning => {
            "border-amber-300 bg-amber-50 text-amber-950 dark:border-amber-800 dark:bg-amber-950 dark:text-amber-100"
        }
        Tone::Danger => {
            "border-red-300 bg-red-50 text-red-900 dark:border-red-800 dark:bg-red-950 dark:text-red-100"
        }
    }
}

fn role(tone: Tone) -> &'static str {
    match tone {
        Tone::Warning | Tone::Danger => "alert",
        Tone::Info | Tone::Success => "status",
    }
}

/// The path of the tone's icon (24 x 24, stroked).
fn icon_path(tone: Tone) -> &'static str {
    match tone {
        Tone::Info => "M12 16v-4m0-4h.01M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0z",
        Tone::Success => "M9 12l2 2 4-4m6 2a9 9 0 1 1-18 0 9 9 0 0 1 18 0z",
        Tone::Warning => {
            "M12 9v4m0 4h.01M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z"
        }
        Tone::Danger => "M12 8v4m0 4h.01M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0z",
    }
}

/// A message box holding `body` (an `html!` fragment).
pub fn alert(options: &AlertOptions<'_>, body: Markup) -> Markup {
    let classes = format!(
        "flex gap-3 rounded-lg border p-4 text-sm {}",
        tone_classes(options.tone)
    );
    html! {
        <div role={role(options.tone)} class={classes}>
            <svg class="mt-0.5 size-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d={icon_path(options.tone)}></path>
            </svg>
            <div class="space-y-1">
                @if let Some(title) = options.title {
                    <p class="font-semibold">{title}</p>
                }
                <div>{body}</div>
            </div>
        </div>
    }
}
