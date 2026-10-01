//! Markdown image-beacon classification for the outbound guardrail.
//!
//! Images are read the way CommonMark renders them: inline destinations,
//! full, collapsed and shortcut references with their link reference
//! definitions, backslash escapes and entity or numeric character references.
//! Whatever this bounded reader cannot classify counts as a remote image.

/// Most Markdown images inspected individually; more are treated as remote.
pub(super) const MAX_INSPECTED_IMAGES: usize = 64;

/// Most `]:` link reference definition candidates read; more are treated as remote.
const MAX_DEFINITIONS: usize = 64;

/// CommonMark's longest link label.
const MAX_LABEL_CHARS: usize = 999;

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

/// Whether the lowercased `text` contains a Markdown image that may load a
/// remote resource, or one this check cannot classify.
///
/// Inline images are judged by their destination. Reference images are judged
/// by the matching definitions; an image with no matching definition (or whose
/// label never closes) is remote when the text also names a remote
/// destination, because the definition may be completed by whoever renders it.
pub(super) fn has_remote_image(text: &str) -> bool {
    if !text.contains("![") {
        return false;
    }
    let Some(definitions) = Definitions::read(text) else {
        return true;
    };
    let mut unresolved = false;
    for (index, (start, _)) in text.match_indices("![").enumerate() {
        if index >= MAX_INSPECTED_IMAGES {
            return true;
        }
        match classify(&text[start + 2..], &definitions) {
            Image::Remote => return true,
            Image::Unresolved => unresolved = true,
            Image::Local | Image::Literal => {}
        }
    }
    unresolved && (definitions.any_remote() || mentions_remote_url(text))
}

enum Image {
    Local,
    Remote,
    /// A reference without a matching definition, or a label that never closes.
    Unresolved,
    /// An empty shortcut such as `![]`, which CommonMark never renders as an image.
    Literal,
}

/// Classifies the image whose label starts at `label`.
fn classify(label: &str, definitions: &Definitions) -> Image {
    let end = match label_end(label) {
        LabelEnd::At(end) => end,
        LabelEnd::Open => return Image::Unresolved,
        LabelEnd::Ambiguous => return Image::Remote,
    };
    let (label, after) = (&label[..end], &label[end + 1..]);
    if let Some(inline) = after.strip_prefix('(') {
        // Invalid inline syntax leaves a shortcut reference, so the label
        // must not name a remote definition either.
        let Some(close) = inline.find(')') else {
            return Image::Remote;
        };
        return if local_destination(&inline[..close])
            && !matches!(definitions.resolve(label, false), Resolution::Remote)
        {
            Image::Local
        } else {
            Image::Remote
        };
    }
    // `![a][b]` uses `b`, `![a][]` and `![a]` use `a`; both are checked.
    let reference = after
        .strip_prefix('[')
        .and_then(|reference| reference.get(..reference.find(']')?));
    let resolutions = [Some(label), reference]
        .into_iter()
        .flatten()
        .filter(|label| !label.trim().is_empty())
        .map(|label| definitions.resolve(label, true))
        .collect::<Vec<_>>();
    if resolutions.is_empty() {
        Image::Literal
    } else if resolutions.contains(&Resolution::Remote) {
        Image::Remote
    } else if resolutions.contains(&Resolution::Local) {
        Image::Local
    } else {
        Image::Unresolved
    }
}

enum LabelEnd {
    /// Byte index of the closing `]`.
    At(usize),
    /// The label never closes.
    Open,
    /// A code span, autolink or raw HTML may move where the label closes.
    Ambiguous,
}

fn label_end(label: &str) -> LabelEnd {
    let mut depth = 0usize;
    let mut escaped = false;
    for (index, character) in label.char_indices() {
        match character {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '`' | '<' => return LabelEnd::Ambiguous,
            '[' => depth += 1,
            ']' if depth == 0 => return LabelEnd::At(index),
            ']' => depth -= 1,
            _ => {}
        }
    }
    LabelEnd::Open
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Resolution {
    Missing,
    Local,
    Remote,
}

struct Definition {
    label: String,
    local: bool,
}

/// Every `[label]: destination` candidate, wherever it appears. Reading more
/// than CommonMark accepts (inside quotes, list items or code) only adds
/// candidates, and a reference is local only when all of its candidates are.
struct Definitions(Vec<Definition>);

impl Definitions {
    /// `None` when the text has more candidates than are inspected.
    fn read(text: &str) -> Option<Self> {
        let mut definitions = Vec::new();
        for (count, (colon, _)) in text.match_indices("]:").enumerate() {
            if count >= MAX_DEFINITIONS {
                return None;
            }
            let Some(start) = text[..colon]
                .char_indices()
                .rev()
                .take(MAX_LABEL_CHARS + 1)
                .find(|(_, character)| *character == '[')
                .map(|(index, _)| index + 1)
            else {
                continue;
            };
            let label = &text[start..colon];
            let destination = definition_destination(&text[colon + 2..]);
            if label.trim().is_empty() || destination.is_empty() {
                continue;
            }
            definitions.push(Definition {
                label: normalize_label(label),
                local: local_destination(destination),
            });
        }
        Some(Self(definitions))
    }

    fn any_remote(&self) -> bool {
        self.0.iter().any(|definition| !definition.local)
    }

    /// Resolves `label` against every candidate. With `fold`, a non-ASCII
    /// label matches any candidate, because CommonMark compares Unicode case
    /// folds (`ß` matches `SS`), which lowercasing does not reproduce.
    fn resolve(&self, label: &str, fold: bool) -> Resolution {
        let label = normalize_label(label);
        let mut resolution = Resolution::Missing;
        for definition in &self.0 {
            let matches = definition.label == label
                || (fold && !(label.is_ascii() && definition.label.is_ascii()));
            if matches {
                if !definition.local {
                    return Resolution::Remote;
                }
                resolution = Resolution::Local;
            }
        }
        resolution
    }
}

fn normalize_label(label: &str) -> String {
    label.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The destination after `]:`: optional whitespace (a definition may continue
/// on the next line), then `<...>` up to its `>` or a run without whitespace.
fn definition_destination(text: &str) -> &str {
    let text = text.trim_start();
    let end = if text.starts_with('<') {
        text.find(['>', '\n']).map_or(text.len(), |end| end + 1)
    } else {
        text.find(char::is_whitespace).unwrap_or(text.len())
    };
    &text[..end]
}

/// Whether the text, once decoded, names a remote destination: an `http:` or
/// `https:` scheme, any `scheme://`, or a `//host` (or backslash) authority
/// with a dotted or bracketed host at the start of a token. Code comments such
/// as `// note`, `//TODO`, `///` and `//!` do not count.
fn mentions_remote_url(text: &str) -> bool {
    let decoded = decode_lenient(text).to_lowercase();
    if decoded.contains("http:") || decoded.contains("https:") || decoded.contains("://") {
        return true;
    }
    let mut previous = ' ';
    for (index, character) in decoded.char_indices() {
        if is_slash(character)
            && !previous.is_alphanumeric()
            && !is_slash(previous)
            && previous != ':'
            && decoded[index + 1..].starts_with(is_slash)
            && names_host(&decoded[index + 2..])
        {
            return true;
        }
        previous = character;
    }
    false
}

const fn is_slash(character: char) -> bool {
    matches!(character, '/' | '\\')
}

/// Whether the authority at the start of `text` is a bracketed address or a
/// dotted name (URL parsers also accept ideographic full stops as dots).
fn names_host(text: &str) -> bool {
    text.starts_with('[')
        || text
            .chars()
            .take_while(|character| {
                !character.is_whitespace()
                    && !is_slash(*character)
                    && !matches!(character, '?' | '#' | ')' | '>' | '"' | '\'')
            })
            .take(255)
            .any(|character| matches!(character, '.' | '\u{3002}' | '\u{ff0e}' | '\u{ff61}'))
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

/// [`decode`] that leaves an unknown named reference as written.
fn decode_lenient(text: &str) -> String {
    decode_with(text, true).unwrap_or_else(|| text.to_string())
}

/// Applies CommonMark backslash escapes and character references, or `None`
/// for a named reference outside [`NAMED_REFERENCES`].
fn decode(text: &str) -> Option<String> {
    decode_with(text, false)
}

fn decode_with(text: &str, keep_unknown: bool) -> Option<String> {
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
        } else if character == '&' {
            match character_reference(rest) {
                Some(Some((reference, length))) => {
                    value = reference;
                    consumed = length;
                }
                Some(None) => {}
                None if keep_unknown => {}
                None => return None,
            }
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
