//! Translations checked at compile time, locale negotiation and a locale
//! switch.
//!
//! An application keeps one TOML catalog per locale (`locales/en.toml`,
//! `locales/pt-BR.toml`) and loads them with [`catalog!`], which fails the
//! build when a key is missing from a locale or its placeholders differ, and
//! defines `LOCALES` and a `t!` macro next to it:
//!
//! ```rust,ignore
//! // src/i18n.rs
//! rullst::i18n::catalog!(default = "en");
//!
//! // src/pages.rs
//! use crate::i18n::{t, LOCALES};
//! use rullst::i18n::Locale;
//!
//! async fn dashboard(locale: Locale, htmx: rullst::HtmxRequest) -> rullst::response::Html<String> {
//!     let body = rullst::html! {
//!         <h1>{t!(locale, "dashboard.greeting", name = "Ana")}</h1>
//!         <p>{t!(locale, "inbox.unread", count = 3)}</p>
//!     };
//!     rullst::render_page_with_lang(&htmx, locale.tag(), &t!(locale, "dashboard.title"), body)
//! }
//!
//! let app = rullst::Router::new()
//!     .route("/", rullst::web::axum::routing::get(dashboard))
//!     .route("/locale", LOCALES.switch_route())
//!     .layer(LOCALES.layer());
//! ```
//!
//! `t!` returns plain text (`Cow<'static, str>`) that `html!` escapes; only
//! keys ending in `_html` are trusted HTML, returned as [`Markup`] with their
//! arguments escaped. The [`Locale`] extractor reads the locale that
//! [`Locales::layer`] negotiated: the [`LOCALE_COOKIE`] cookie, then
//! `Accept-Language`, then the default. [`Locales::switch_route`] is a
//! `POST` handler that stores a supported locale in that cookie; it relies on
//! the application's CSRF middleware like every other form.
//!
//! The example is compiled and run in `rullst-core/tests/i18n_catalog.rs`.
//!
//! [`Markup`]: crate::html::Markup

use std::fmt;

mod http;
mod negotiate;
mod render;

pub use http::{LocaleLayer, LocaleService, MissingLocale};
#[doc(hidden)]
pub use render::{__Catalog, __Forms, __Plural, __Segment, __html, __locale, __text};
#[doc(hidden)]
pub use rullst_macros::__i18n_translate as __translate;
#[doc(inline)]
pub use rullst_macros::i18n_catalog as catalog;

/// Cookie that stores the locale chosen with [`Locales::switch_route`].
pub const LOCALE_COOKIE: &str = "rullst_locale";

/// Most locales in one [`Locales`] list.
const MAX_LOCALES: usize = 64;

/// One of the application's supported locales, identified by its BCP 47
/// tag (`en`, `pt-BR`).
///
/// A `Locale` is obtained from [`Locales`] (negotiation, [`Locales::get`])
/// or extracted in a handler after [`Locales::layer`]; pass
/// [`tag`](Self::tag) to `render_page_with_lang` for `<html lang>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Locale {
    tag: &'static str,
}

impl Locale {
    /// The locale's tag, such as `pt-BR`.
    pub fn tag(&self) -> &'static str {
        self.tag
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.tag)
    }
}

/// The locales an application supports and the default one.
///
/// [`catalog!`] generates a `static LOCALES: Locales` from the catalog
/// files; [`Locales::new`] builds a list for an application that keeps its
/// messages elsewhere but wants the negotiation and the switch route.
#[derive(Debug)]
pub struct Locales {
    tags: &'static [&'static str],
    default: usize,
}

/// Why [`Locales::new`] rejected a locale list.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LocaleError {
    /// The list is empty.
    #[error("a locale list needs at least one tag")]
    Empty,
    /// The list has more than 64 tags.
    #[error("a locale list holds at most 64 tags")]
    TooMany,
    /// A tag is not `language[-Script][-REGION]` in canonical case.
    #[error("`{0}` is not a canonical `language[-Script][-REGION]` tag")]
    InvalidTag(String),
    /// A tag appears twice.
    #[error("`{0}` is listed twice")]
    DuplicateTag(String),
    /// The default tag is not in the list.
    #[error("the default locale `{0}` is not in the list")]
    UnknownDefault(String),
}

impl Locales {
    /// Validates a list of canonical tags (`en`, `pt-BR`, `zh-Hant`,
    /// `es-419`) and the default one.
    pub fn new(tags: &'static [&'static str], default: &str) -> Result<Self, LocaleError> {
        if tags.is_empty() {
            return Err(LocaleError::Empty);
        }
        if tags.len() > MAX_LOCALES {
            return Err(LocaleError::TooMany);
        }
        for (index, tag) in tags.iter().enumerate() {
            if !is_canonical_tag(tag) {
                return Err(LocaleError::InvalidTag((*tag).to_owned()));
            }
            if tags.iter().take(index).any(|earlier| earlier == tag) {
                return Err(LocaleError::DuplicateTag((*tag).to_owned()));
            }
        }
        let default = tags
            .iter()
            .position(|tag| *tag == default)
            .ok_or_else(|| LocaleError::UnknownDefault(default.chars().take(64).collect()))?;
        Ok(Self { tags, default })
    }

    /// Used by `catalog!`, which validated the tags at compile time.
    #[doc(hidden)]
    pub const fn __from_catalog(tags: &'static [&'static str], default: usize) -> Self {
        Self { tags, default }
    }

    /// The default locale.
    pub fn default_locale(&self) -> Locale {
        let tag = self.tags.get(self.default).or(self.tags.first());
        Locale {
            tag: tag.copied().unwrap_or("en"),
        }
    }

    /// The supported locale with this tag, compared ignoring ASCII case.
    pub fn get(&self, tag: &str) -> Option<Locale> {
        self.tags
            .iter()
            .copied()
            .find(|supported| supported.eq_ignore_ascii_case(tag))
            .map(|tag| Locale { tag })
    }

    /// Every supported locale, in catalog order (for a language menu).
    pub fn iter(&self) -> impl Iterator<Item = Locale> + use<> {
        self.tags.iter().copied().map(|tag| Locale { tag })
    }
}

/// `language[-Script][-REGION]` in canonical case: `en`, `zh-Hant`,
/// `pt-BR`, `es-419`.
fn is_canonical_tag(tag: &str) -> bool {
    let mut subtags = tag.split('-').peekable();
    let language = subtags.next().unwrap_or_default();
    let script = |s: &&str| {
        let mut letters = s.bytes();
        s.len() == 4
            && letters.next().is_some_and(|b| b.is_ascii_uppercase())
            && letters.all(|b| b.is_ascii_lowercase())
    };
    let region = |s: &str| {
        (s.len() == 2 && s.bytes().all(|b| b.is_ascii_uppercase()))
            || (s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
    };
    if !(2..=3).contains(&language.len()) || !language.bytes().all(|b| b.is_ascii_lowercase()) {
        return false;
    }
    let _script = subtags.next_if(script);
    match subtags.next() {
        None => true,
        Some(subtag) => region(subtag) && subtags.next().is_none(),
    }
}

/// Integers that select a plural form: `t!(locale, "inbox.unread", count = n)`.
///
/// The form is chosen from the absolute value; values beyond `u64` count
/// as `u64::MAX`.
pub trait PluralCount {
    /// The absolute value as `u64`, saturating.
    fn plural_count(&self) -> u64;
}

macro_rules! plural_count {
    ($($ty:ty => |$value:ident| $body:expr),* $(,)?) => {
        $(impl PluralCount for $ty {
            fn plural_count(&self) -> u64 {
                let $value = *self;
                $body
            }
        })*
    };
}

plural_count! {
    u8 => |n| u64::from(n),
    u16 => |n| u64::from(n),
    u32 => |n| u64::from(n),
    u64 => |n| n,
    u128 => |n| u64::try_from(n).unwrap_or(u64::MAX),
    usize => |n| u64::try_from(n).unwrap_or(u64::MAX),
    i8 => |n| u64::from(n.unsigned_abs()),
    i16 => |n| u64::from(n.unsigned_abs()),
    i32 => |n| u64::from(n.unsigned_abs()),
    i64 => |n| n.unsigned_abs(),
    i128 => |n| u64::try_from(n.unsigned_abs()).unwrap_or(u64::MAX),
    isize => |n| u64::try_from(n.unsigned_abs()).unwrap_or(u64::MAX),
}

impl<T: PluralCount + ?Sized> PluralCount for &T {
    fn plural_count(&self) -> u64 {
        (**self).plural_count()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "tests.rs"]
mod tests;
