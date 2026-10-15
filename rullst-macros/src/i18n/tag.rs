//! Locale tags (a BCP 47 subset) and the plural rule chosen for each one.

/// How a locale chooses between the `one` and `other` plural forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PluralRule {
    /// `one` for exactly 1: en, es, de, it, nl and pt-PT.
    One,
    /// `one` for 0 and 1: pt (pt, pt-BR) and fr.
    ZeroOne,
    /// No `one` form: ja, zh and ko.
    Other,
}

/// Languages with a plural rule, listed in compile errors.
pub(crate) const PLURAL_LANGUAGES: &str = "en, es, de, it, nl, fr, pt, pt-BR, pt-PT, ja, zh, ko";

impl PluralRule {
    /// The rule for a canonical tag, or `None` when Rullst has no
    /// documented rule for its language (or Portuguese region).
    pub(crate) fn for_tag(tag: &str) -> Option<Self> {
        let mut subtags = tag.split('-');
        let language = subtags.next().unwrap_or_default();
        let region = subtags.find(|subtag| subtag.len() != 4);
        match language {
            "en" | "es" | "de" | "it" | "nl" => Some(Self::One),
            "fr" => Some(Self::ZeroOne),
            "pt" => match region {
                None | Some("BR") => Some(Self::ZeroOne),
                Some("PT") => Some(Self::One),
                Some(_) => None,
            },
            "ja" | "zh" | "ko" => Some(Self::Other),
            _ => None,
        }
    }

    /// Whether the rule has a `one` form that catalogs must provide.
    pub(crate) fn has_one(self) -> bool {
        self != Self::Other
    }

    /// Name of the matching `rullst::i18n::__Plural` variant.
    pub(crate) fn variant(self) -> &'static str {
        match self {
            Self::One => "One",
            Self::ZeroOne => "ZeroOne",
            Self::Other => "Other",
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
