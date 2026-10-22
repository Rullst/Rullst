//! Locale tags (a BCP 47 subset) and the plural rule chosen for each one.

/// A locale's CLDR cardinal plural rule, by the forms it selects. The
/// runtime twin is `rullst::i18n::__Plural`; the variant names must match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PluralRule {
    /// `other` only: ja, zh, ko.
    Other,
    /// `one` for 1 without visible fraction digits: en, de, nl.
    One,
    /// [`One`](Self::One) plus `many` for exact millions: it, pt-PT.
    OneMany,
    /// `one` below 2 (`i = 0,1`) plus `many` for exact millions: fr, pt.
    ZeroOne,
    /// `one` for the value 1 (`1.0` too): tr.
    ExactOne,
    /// [`ExactOne`](Self::ExactOne) plus `many` for exact millions: es.
    ExactOneMany,
    /// `one`, `few`, `many`, `other` by the last digits: ru, uk.
    EastSlavic,
    /// `one`, `few`, `many`, `other`: pl.
    Polish,
    /// `one`, `few` (2–4), `many` (decimals), `other`: cs.
    Czech,
    /// `one`, `few`, `other`: ro.
    Romanian,
    /// `zero`, `one`, `two`, `few`, `many`, `other`: ar.
    Arabic,
    /// `one`, `two`, `other`: he.
    Hebrew,
}

/// Languages with a plural rule, listed in compile errors.
pub(crate) const PLURAL_LANGUAGES: &str =
    "ar, cs, de, en, es, fr, he, it, ja, ko, nl, pl, pt, pt-BR, pt-PT, ro, ru, tr, uk, zh";

impl PluralRule {
    /// The rule for a canonical tag, or `None` when Rullst has no
    /// documented rule for its language (or Portuguese region).
    pub(crate) fn for_tag(tag: &str) -> Option<Self> {
        let mut subtags = tag.split('-');
        let language = subtags.next().unwrap_or_default();
        let region = subtags.find(|subtag| subtag.len() != 4);
        match language {
            "en" | "de" | "nl" => Some(Self::One),
            "it" => Some(Self::OneMany),
            "fr" => Some(Self::ZeroOne),
            "pt" => match region {
                None | Some("BR") => Some(Self::ZeroOne),
                Some("PT") => Some(Self::OneMany),
                Some(_) => None,
            },
            "tr" => Some(Self::ExactOne),
            "es" => Some(Self::ExactOneMany),
            "ru" | "uk" => Some(Self::EastSlavic),
            "pl" => Some(Self::Polish),
            "cs" => Some(Self::Czech),
            "ro" => Some(Self::Romanian),
            "ar" => Some(Self::Arabic),
            "he" => Some(Self::Hebrew),
            "ja" | "zh" | "ko" => Some(Self::Other),
            _ => None,
        }
    }

    /// The forms a plural message must define, in CLDR order.
    pub(crate) fn required(self) -> &'static [&'static str] {
        match self {
            Self::Other => &["other"],
            Self::One | Self::OneMany | Self::ZeroOne | Self::ExactOne | Self::ExactOneMany => {
                &["one", "other"]
            }
            Self::EastSlavic | Self::Polish | Self::Czech => &["one", "few", "many", "other"],
            Self::Romanian => &["one", "few", "other"],
            Self::Arabic => &["zero", "one", "two", "few", "many", "other"],
            Self::Hebrew => &["one", "two", "other"],
        }
    }

    /// Whether the rule also selects `many` (exact millions), which a
    /// message may leave out: `other` is used then.
    pub(crate) fn optional_many(self) -> bool {
        matches!(self, Self::OneMany | Self::ZeroOne | Self::ExactOneMany)
    }

    /// Whether a plural message may define `category`.
    pub(crate) fn allows(self, category: &str) -> bool {
        self.required().contains(&category) || (category == "many" && self.optional_many())
    }

    /// Name of the matching `rullst::i18n::__Plural` variant.
    pub(crate) fn variant(self) -> &'static str {
        match self {
            Self::Other => "Other",
            Self::One => "One",
            Self::OneMany => "OneMany",
            Self::ZeroOne => "ZeroOne",
            Self::ExactOne => "ExactOne",
            Self::ExactOneMany => "ExactOneMany",
            Self::EastSlavic => "EastSlavic",
            Self::Polish => "Polish",
            Self::Czech => "Czech",
            Self::Romanian => "Romanian",
            Self::Arabic => "Arabic",
            Self::Hebrew => "Hebrew",
        }
    }
}

/// Returns the canonical spelling of `language[-Script][-REGION]`
/// (`pt-BR`, `zh-Hant`, `es-419`), or `None` for anything else.
pub(crate) fn canonical(tag: &str) -> Option<String> {
    let mut subtags = tag.split('-');
    let language = subtags.next()?;
    if !(2..=3).contains(&language.len()) || !language.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let mut canonical = language.to_ascii_lowercase();
    let mut rest = subtags.peekable();
    if let Some(script) =
        rest.next_if(|s| s.len() == 4 && s.bytes().all(|b| b.is_ascii_alphabetic()))
    {
        canonical.push('-');
        canonical.push_str(&script[..1].to_ascii_uppercase());
        canonical.push_str(&script[1..].to_ascii_lowercase());
    }
    if let Some(region) = rest.next() {
        let letters = region.len() == 2 && region.bytes().all(|b| b.is_ascii_alphabetic());
        let digits = region.len() == 3 && region.bytes().all(|b| b.is_ascii_digit());
        if !letters && !digits {
            return None;
        }
        canonical.push('-');
        canonical.push_str(&region.to_ascii_uppercase());
    }
    rest.next().is_none().then_some(canonical)
}

/// Checks a catalog file's tag, suggesting the canonical spelling.
pub(crate) fn check(tag: &str) -> Result<(), String> {
    match canonical(tag) {
        Some(canonical) if canonical == tag => Ok(()),
        Some(canonical) => Err(format!(
            "locale `{tag}` is not in canonical form; name the file `{canonical}.toml`"
        )),
        None => Err(format!(
            "`{tag}` is not a supported locale tag: use `language[-Script][-REGION]`, \
             such as `en`, `pt-BR`, `zh-Hant` or `es-419`"
        )),
    }
}
