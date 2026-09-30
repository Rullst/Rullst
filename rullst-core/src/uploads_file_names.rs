//! Characters a validated display-only upload name must not contain.

/// Unicode format characters (general category Cf, Unicode 15) and the
/// line/paragraph separators. Bidi controls such as U+202E reorder how a name
/// displays, so `report\u{202E}fdp.png` is admitted as PNG but reads as
/// `reportgnp.pdf`; the other format characters are invisible. ZWNJ (U+200C)
/// and ZWJ (U+200D) stay allowed because scripts and emoji sequences need
/// them and they do not reorder text.
pub(super) fn is_hidden_format(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'
            | '\u{200E}'..='\u{200F}'
            | '\u{2028}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}
