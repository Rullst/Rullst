//! Card, copied by `cargo rullst ui add card`; the application owns this
//! file. A bordered surface with an optional heading, description and footer.
//!
//! ```text
//! use crate::ui::card::{card, CardOptions, Heading};
//!
//! card(
//!     &CardOptions { title: Some("Plan"), description: Some("Billed monthly"), ..Default::default() },
//!     html! { <p>"Pro: 20 seats"</p> },
//!     None,
//! )
//! ```
//!
//! Accessibility: pick the heading level that continues the page's outline
//! (`H2` below the page's `h1`, `H3` inside a section). A titled card is a
//! `<section>` named by its heading.

use rullst::html;
use rullst::html::Markup;

/// The level of the card's heading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Heading {
    #[default]
    H2,
    H3,
    H4,
}

/// Options of [`card`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CardOptions<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub heading: Heading,
    /// The `id` of the card; the heading's is `<id>-title`. Needed when a
    /// page shows several titled cards, so that each heading id is unique.
    pub id: Option<&'a str>,
}

const SURFACE: &str = "rounded-xl border border-slate-200 bg-white text-slate-900 shadow-sm dark:border-slate-800 dark:bg-slate-900 dark:text-slate-100";
const TITLE: &str = "text-lg leading-tight font-semibold";

fn title_markup(level: Heading, id: &str, title: &str) -> Markup {
    html! {
        @match level {
            Heading::H2 => { <h2 id={id} class={TITLE}>{title}</h2> }
            Heading::H3 => { <h3 id={id} class={TITLE}>{title}</h3> }
            Heading::H4 => { <h4 id={id} class={TITLE}>{title}</h4> }
        }
    }
}

/// A card holding `body`, with `footer` (usually actions) below it.
pub fn card(options: &CardOptions<'_>, body: Markup, footer: Option<Markup>) -> Markup {
    let base = options.id.unwrap_or("card");
    let title_id = format!("{base}-title");
    let header = options.title.map(|title| {
        html! {
            <div class="space-y-1.5 px-6 pt-6">
                {title_markup(options.heading, &title_id, title)}
                @if let Some(description) = options.description {
                    <p class="text-sm text-slate-600 dark:text-slate-400">{description}</p>
                }
            </div>
        }
    });
    let footer = footer.map(|footer| {
        html! {
            <div class="flex items-center justify-end gap-2 border-t border-slate-200 px-6 py-4 dark:border-slate-800">{footer}</div>
        }
    });
    let content = html! {
        {header}
        <div class="px-6 py-6">{body}</div>
        {footer}
    };
    html! {
        @if options.title.is_some() {
            <section id?={options.id} aria-labelledby={title_id.as_str()} class={SURFACE}>{content}</section>
        } @else {
            <div id?={options.id} class={SURFACE}>{content}</div>
        }
    }
}
