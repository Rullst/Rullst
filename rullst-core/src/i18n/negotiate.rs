//! Choosing a request's locale: the locale cookie, then `Accept-Language`,
//! then the default.

use super::{LOCALE_COOKIE, Locale, Locales};
use axum::http::{HeaderMap, header};

/// Longest `Accept-Language` value read; longer headers are ignored.
const MAX_ACCEPT_LANGUAGE_BYTES: usize = 4096;
/// Language ranges read from one `Accept-Language` header.
const MAX_LANGUAGE_RANGES: usize = 32;
/// Longest language tag or range considered (RFC 5646's practical limit).
const MAX_TAG_BYTES: usize = 35;

impl Locales {
    /// The request's locale: a supported tag in the [`LOCALE_COOKIE`]
    /// cookie, else the best `Accept-Language` match, else the default.
    pub fn negotiate(&self, headers: &HeaderMap) -> Locale {
        self.cookie_locale(headers)
            .or_else(|| {
                let value = headers.get(header::ACCEPT_LANGUAGE)?.to_str().ok()?;
                self.accept_language(value)
            })
            .unwrap_or_else(|| self.default_locale())
    }

    /// The first supported tag stored in the [`LOCALE_COOKIE`] cookie.
    pub fn cookie_locale(&self, headers: &HeaderMap) -> Option<Locale> {
        let name = LOCALE_COOKIE.as_bytes();
        headers
            .get_all(header::COOKIE)
            .iter()
            .flat_map(|value| value.as_bytes().split(|byte| *byte == b';'))
            .filter_map(|pair| pair.trim_ascii().strip_prefix(name)?.strip_prefix(b"="))
            .find_map(|value| self.get(std::str::from_utf8(value).ok()?))
    }

    /// The supported locale that best matches an `Accept-Language` value.
    ///
    /// Ranges are tried by descending weight (`q`, RFC 9110), keeping header
    /// order for equal weights. A range matches a supported tag exactly
    /// (ignoring case) or, failing that, by language (`pt-PT` and `pt` match
    /// `pt-BR`, preferring the default locale when it has that language);
    /// `*` matches the default. Ranges with `q=0` or a malformed weight are
    /// skipped. Only the first 32 ranges of a value up to 4 KiB are read.
    pub fn accept_language(&self, value: &str) -> Option<Locale> {
        if value.len() > MAX_ACCEPT_LANGUAGE_BYTES {
            return None;
        }
        let mut ranges: Vec<(u16, &str)> = value
            .split(',')
            .take(MAX_LANGUAGE_RANGES)
            .filter_map(language_range)
            .collect();
        // A stable sort keeps the header order among equal weights.
        ranges.sort_by_key(|(weight, _)| std::cmp::Reverse(*weight));
        ranges.into_iter().find_map(|(_, range)| {
            if range == "*" {
                return Some(self.default_locale());
            }
            let index = closest(self.tags, self.default, range)?;
            self.tags.get(index).copied().map(|tag| Locale { tag })
        })
    }
}

/// Parses `en-GB;q=0.8` into its weight in thousandths and its range.
fn language_range(item: &str) -> Option<(u16, &str)> {
    let mut parts = item.split(';');
    let range = parts.next().unwrap_or_default().trim();
    if range.is_empty() || range.len() > MAX_TAG_BYTES {
        return None;
    }
    let mut weight = 1000;
    for parameter in parts {
        let (name, value) = parameter.split_once('=')?;
        if name.trim().eq_ignore_ascii_case("q") {
            weight = parse_weight(value.trim())?;
        }
    }
    (weight > 0).then_some((weight, range))
}

/// An RFC 9110 weight, `0[.ddd]` or `1[.000]`, in thousandths.
fn parse_weight(value: &str) -> Option<u16> {
    let (integer, fraction) = value.split_once('.').unwrap_or((value, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let thousandths = fraction
        .bytes()
        .chain(std::iter::repeat(b'0'))
        .take(3)
        .fold(0u16, |total, digit| total * 10 + u16::from(digit - b'0'));
    match integer {
        "0" => Some(thousandths),
        "1" if thousandths == 0 => Some(1000),
        _ => None,
    }
}

/// Index of the tag matching `tag` exactly (ignoring case), else of a tag
/// with the same language (the default first), else `None`.
pub(super) fn closest(tags: &[&str], default: usize, tag: &str) -> Option<usize> {
    if let Some(index) = tags.iter().position(|t| t.eq_ignore_ascii_case(tag)) {
        return Some(index);
    }
    let wanted = language(tag);
    let same = |t: &&str| language(t).eq_ignore_ascii_case(wanted);
    if tags.get(default).is_some_and(same) {
        return Some(default);
    }
    tags.iter().position(same)
}

fn language(tag: &str) -> &str {
    tag.split('-').next().unwrap_or(tag)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "negotiate_tests.rs"]
mod tests;
