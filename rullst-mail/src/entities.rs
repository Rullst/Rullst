//! Single-pass HTML character-reference decoding.
//!
//! Browsers decode references in text and attribute values before they act
//! on them, so link checks and derived text must see the decoded value.
//! Numeric references (`&#58;`, `&#x3a;`, with or without the trailing `;`)
//! and the named references below are decoded; anything else stays literal.
//! Each reference is decoded once, so `&amp;lt;` becomes `&lt;`, not `<`.

use std::borrow::Cow;

/// Named references decoded by Rullst. Names are case-sensitive, as in HTML.
/// `&nbsp;` becomes an ordinary space, as the plain-text fallback always did.
const NAMED: &[(&str, &str)] = &[
    ("amp", "&"),
    ("lt", "<"),
    ("gt", ">"),
    ("quot", "\""),
    ("apos", "'"),
    ("nbsp", " "),
    ("colon", ":"),
    ("Tab", "\t"),
    ("NewLine", "\n"),
    ("bull", "\u{2022}"),
    ("middot", "\u{b7}"),
    ("hellip", "\u{2026}"),
    ("ndash", "\u{2013}"),
    ("mdash", "\u{2014}"),
    ("lsquo", "\u{2018}"),
    ("rsquo", "\u{2019}"),
    ("ldquo", "\u{201c}"),
    ("rdquo", "\u{201d}"),
    ("laquo", "\u{ab}"),
    ("raquo", "\u{bb}"),
    ("copy", "\u{a9}"),
    ("reg", "\u{ae}"),
    ("trade", "\u{2122}"),
    ("euro", "\u{20ac}"),
    ("times", "\u{d7}"),
    ("deg", "\u{b0}"),
];

/// Longest name in [`NAMED`], which bounds the lookahead for `;`.
const MAX_NAME_BYTES: usize = 7;

/// Decodes character references in one forward pass.
pub(crate) fn decode(input: &str) -> Cow<'_, str> {
    if !input.contains('&') {
        return Cow::Borrowed(input);
    }
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(index) = rest.find('&') {
        output.push_str(&rest[..index]);
        let tail = &rest[index + 1..];
        match decode_reference(tail, &mut output) {
            Some(consumed) => rest = &tail[consumed..],
            None => {
                output.push('&');
                rest = tail;
            }
        }
    }
    output.push_str(rest);
    Cow::Owned(output)
}

/// Appends the reference at the start of `tail` (just after `&`) and returns
/// the bytes it used, or `None` when `tail` does not start a known reference.
fn decode_reference(tail: &str, output: &mut String) -> Option<usize> {
    let bytes = tail.as_bytes();
    if bytes.first() == Some(&b'#') {
        let hex = matches!(bytes.get(1), Some(b'x' | b'X'));
        let digits_start = if hex { 2 } else { 1 };
        let radix = if hex { 16 } else { 10 };
        let digits = bytes[digits_start..]
            .iter()
            .take_while(|byte| (**byte as char).is_digit(radix))
            .count();
        if digits == 0 {
            return None;
        }
        let value = bytes[digits_start..digits_start + digits]
            .iter()
            .filter_map(|byte| (*byte as char).to_digit(radix))
            .fold(0u32, |value, digit| {
                value
                    .saturating_mul(radix)
                    .saturating_add(digit)
                    .min(0x11_0000)
            });
        let character = char::from_u32(value)
            .filter(|character| *character != '\0')
            .unwrap_or('\u{fffd}');
        output.push(character);
        let end = digits_start + digits;
        return Some(if bytes.get(end) == Some(&b';') {
            end + 1
        } else {
            end
        });
    }
    let semicolon = bytes
        .iter()
        .take(MAX_NAME_BYTES + 1)
        .position(|byte| *byte == b';')?;
    let name = tail.get(..semicolon)?;
    let (_, decoded) = NAMED.iter().find(|(candidate, _)| *candidate == name)?;
    output.push_str(decoded);
    Some(semicolon + 1)
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_numeric_and_named_references_once() {
        assert_eq!(decode("plain"), "plain");
        assert_eq!(decode("a=1&amp;b=2"), "a=1&b=2");
        assert_eq!(decode("O&#x27;Brien &#39;s &apos;"), "O'Brien 's '");
        assert_eq!(decode("javascript&colon;x"), "javascript:x");
        assert_eq!(decode("java&#x09;script&#0000058x"), "java\tscript:x");
        assert_eq!(
            decode("Paid &bull; Billing&nbsp;OK"),
            "Paid \u{2022} Billing OK"
        );
        assert_eq!(decode("&amp;lt;b&amp;gt;"), "&lt;b&gt;");
        assert_eq!(
            decode("&#0;&#xD800;&#99999999;"),
            "\u{fffd}\u{fffd}\u{fffd}"
        );
        for literal in ["&", "& b", "&unknown;", "&#;", "&#x;", "&AMP;", "&amp"] {
            assert_eq!(decode(literal), literal);
        }
    }
}
