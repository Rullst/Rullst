//! Invisible characters that can hide or split prompt text.
//!
//! The classification matches the `rullst-ai` guardrail: controls with no
//! ordinary use in prompts are blocked, while default-ignorable characters
//! that also occur in ordinary text are only removed before phrase matching.

/// Invisible default-ignorable code points with no ordinary use in prompts.
///
/// Covers zero-width characters and joiners, bidirectional embeddings,
/// overrides and isolates, the word joiner, invisible operators and
/// deprecated format controls, the combining grapheme joiner, Hangul fillers,
/// reserved default-ignorable code points, shorthand and musical format
/// controls, and the plane-14 tag characters and variation selectors used to
/// smuggle text.
pub(super) const fn is_invisible_control(c: char) -> bool {
    matches!(
        c,
        '\u{034F}'
            | '\u{115F}'..='\u{1160}'
            | '\u{200B}'..='\u{200D}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{3164}'
            | '\u{FEFF}'
            | '\u{FFA0}'
            | '\u{FFF0}'..='\u{FFF8}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0000}'..='\u{E0FFF}'
    )
}

/// Default-ignorable code points that also occur in ordinary text: the soft
/// hyphen, Arabic letter mark, Khmer inherent vowels, Mongolian selectors and
/// vowel separator, left-to-right and right-to-left marks, and the variation
/// selectors used by emoji. They are removed before phrase matching instead of
/// being blocked.
const fn is_ignorable_format(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{061C}'
            | '\u{17B4}'..='\u{17B5}'
            | '\u{180B}'..='\u{180F}'
            | '\u{200E}'..='\u{200F}'
            | '\u{FE00}'..='\u{FE0F}'
    )
}

/// Lowercases `raw`, removes ignorable format characters and collapses every
/// whitespace run to one space, so a soft hyphen, a doubled space or a line
/// break inside a phrase does not defeat matching.
pub(super) fn normalize_for_patterns(raw: &str) -> String {
    let mut normalized = String::with_capacity(raw.len());
    let mut pending_space = false;
    for c in raw
        .chars()
        .filter(|c| !is_ignorable_format(*c))
        .flat_map(char::to_lowercase)
    {
        if c.is_whitespace() {
            pending_space = !normalized.is_empty();
            continue;
        }
        if pending_space {
            normalized.push(' ');
            pending_space = false;
        }
        normalized.push(c);
    }
    normalized
}
