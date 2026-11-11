//! Badge, copied by `cargo rullst ui add badge`; the application owns this
//! file. A short status label. Its text carries the meaning; the colour only
//! repeats it, so it is never the sole signal.
//!
//! ```text
//! use crate::ui::badge::{badge, Tone};
//!
//! badge("Paid", Tone::Success)
//! ```

use rullst::html;
use rullst::html::Markup;

/// The colour of a badge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

fn tone_classes(tone: Tone) -> &'static str {
    match tone {
        Tone::Neutral => {
            "border-slate-300 bg-slate-100 text-slate-800 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200"
        }
        Tone::Info => {
            "border-sky-300 bg-sky-50 text-sky-800 dark:border-sky-800 dark:bg-sky-950 dark:text-sky-200"
        }
        Tone::Success => {
            "border-emerald-300 bg-emerald-50 text-emerald-800 dark:border-emerald-800 dark:bg-emerald-950 dark:text-emerald-200"
        }
        Tone::Warning => {
            "border-amber-300 bg-amber-50 text-amber-900 dark:border-amber-800 dark:bg-amber-950 dark:text-amber-200"
        }
        Tone::Danger => {
            "border-red-300 bg-red-50 text-red-800 dark:border-red-800 dark:bg-red-950 dark:text-red-200"
        }
    }
}

/// The classes of a badge with `tone`.
pub fn classes(tone: Tone) -> String {
    format!(
        "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium {}",
        tone_classes(tone)
    )
}

/// A `<span>` badge reading `text`.
pub fn badge(text: &str, tone: Tone) -> Markup {
    html! {
        <span class={classes(tone)}>{text}</span>
    }
}
