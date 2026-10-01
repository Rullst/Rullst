//! Markdown image-beacon classification for the outbound guardrail.
//!
//! Destinations are read the way CommonMark renders them: backslash escapes
//! and entity or numeric character references are decoded before deciding
//! whether an image can load a remote resource.

/// Most Markdown images inspected individually; more keep the whole-text check.
pub(super) const MAX_INSPECTED_IMAGES: usize = 64;

/// Longest entity name looked up; longer names are not character references.
const MAX_ENTITY_NAME: usize = 32;

/// Named character references decoded in destinations. Names are lowercase
/// because the guardrail inspects lowercased text; any other name is treated
/// as unknown, so the destination is not trusted.
const NAMED_REFERENCES: &[(&str, char)] = &[
    ("amp", '&'),
    ("apos", '\''),
    ("ast", '*'),
    ("bsol", '\\'),
    ("colon", ':'),
    ("comma", ','),
    ("commat", '@'),
    ("dollar", '$'),
    ("equals", '='),
    ("excl", '!'),
    ("gt", '>'),
    ("lowbar", '_'),
    ("lpar", '('),
    ("lt", '<'),
    ("nbsp", '\u{a0}'),
    ("newline", '\n'),
    ("num", '#'),
    ("percnt", '%'),
    ("period", '.'),
    ("plus", '+'),
    ("quest", '?'),
    ("quot", '"'),
    ("rpar", ')'),
    ("semi", ';'),
    ("sol", '/'),
    ("tab", '\t'),
];

/// Whether every Markdown image is inline with a local destination, such as
/// `![logo](assets/logo.png)`, so a URL elsewhere in the text is not an image
/// beacon. Reference-style, unterminated or remote images keep the
/// conservative whole-text check.
pub(super) fn every_image_is_local(text: &str) -> bool {
    let mut inspected = 0usize;
    for (start, _) in text.match_indices("![") {
        inspected += 1;
        if inspected > MAX_INSPECTED_IMAGES
            || !text
                .get(start + 2..)
                .and_then(inline_image_destination)
                .is_some_and(local_destination)
        {
            return false;
        }
    }
    true
}

/// Destination of an inline image whose label starts at `label`.
fn inline_image_destination(label: &str) -> Option<&str> {
    let mut depth = 0usize;
    let mut escaped = false;
    for (index, character) in label.char_indices() {
        match character {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '[' => depth += 1,
            ']' if depth == 0 => {
                let destination = label.get(index + 1..)?.strip_prefix('(')?;
                return destination.get(..destination.find(')')?);
            }
            ']' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// A destination without a scheme (`:`) or authority (a leading `//`, which
/// URL parsers also accept as backslashes), both as written and as CommonMark
/// decodes it. Whitespace, controls and `<` are ignored, as URL parsers strip
/// or drop them. A named reference this check does not know is not local.
fn local_destination(destination: &str) -> bool {
    decode(destination).is_some_and(|decoded| without_scheme_or_authority(&decoded))
        && without_scheme_or_authority(destination)
}

fn without_scheme_or_authority(destination: &str) -> bool {
    let mut characters = destination.chars().filter(|character| {
        !character.is_whitespace() && !character.is_control() && *character != '<'
    });
    let authority = matches!(characters.next(), Some('/' | '\\'))
        && matches!(characters.next(), Some('/' | '\\'));
    !authority && !destination.contains(':')
}

/// Applies CommonMark backslash escapes and character references, or `None`
/// for a named reference outside [`NAMED_REFERENCES`].
fn decode(text: &str) -> Option<String> {
    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(character) = rest.chars().next() {
        let mut consumed = character.len_utf8();
        let mut value = character;
        if character == '\\'
            && let Some(escaped) = rest[1..].chars().next()
            && escaped.is_ascii_punctuation()
        {
            value = escaped;
            consumed = 2;
        } else if character == '&'
            && let Some((reference, length)) = character_reference(rest)?
        {
            value = reference;
            consumed = length;
        }
        decoded.push(value);
        rest = &rest[consumed..];
    }
    Some(decoded)
}

/// Decodes the character reference at the start of `text` (which begins with
/// `&`) as `(character, byte length)`. `Some(None)` means the ampersand is
/// literal; `None` means an unknown named reference.
fn character_reference(text: &str) -> Option<Option<(char, usize)>> {
    let body = &text[1..];
    let Some((end, _)) = body
        .char_indices()
        .take(MAX_ENTITY_NAME + 1)
        .find(|(_, character)| *character == ';')
    else {
        return Some(None);
    };
    let name = &body[..end];
    let length = end + 2;
    if let Some(number) = name.strip_prefix('#') {
        let (digits, radix, max_digits) = match number.strip_prefix(['x', 'X']) {
            Some(hex) => (hex, 16, 6),
            None => (number, 10, 7),
        };
        if digits.is_empty()
            || digits.len() > max_digits
            || !digits.chars().all(|digit| digit.is_digit(radix))
        {
            return Some(None);
        }
        let value = u32::from_str_radix(digits, radix)
            .ok()
            .filter(|value| *value != 0)
            .and_then(char::from_u32)
            .unwrap_or(char::REPLACEMENT_CHARACTER);
        return Some(Some((value, length)));
    }
    let mut characters = name.chars();
    if name.len() > MAX_ENTITY_NAME
        || !characters
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
        || !characters.all(|character| character.is_ascii_alphanumeric())
    {
        return Some(None);
    }
    NAMED_REFERENCES
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, value)| Some((*value, length)))
}
