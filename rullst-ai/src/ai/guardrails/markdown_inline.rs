//! Inline constructs that can move where an image label closes.
//!
//! CommonMark gives code spans, raw HTML and autolinks precedence over link
//! brackets, so a `]` inside one of them does not close a label. Such a
//! construct matters only when it could contain a bracket: these checks
//! answer "might this hide a bracket?" and over-approximate, so a `true` is
//! sometimes unnecessary but a `false` is always safe. Rust code such as
//! `Vec::<u8>` or `Box<dyn Fn()>` inside `vec![...]` therefore no longer
//! makes the label ambiguous.

const fn whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

/// `rest` starts at a backtick run. The run can only hide a bracket when a
/// later run of the same length exists (otherwise it is literal) and the
/// text between them contains `[` or `]`. Searching the whole remaining text
/// over-approximates the paragraph the span would have to close in.
pub(super) fn code_span_hides_bracket(rest: &str) -> bool {
    let bytes = rest.as_bytes();
    let length = bytes.iter().take_while(|byte| **byte == b'`').count();
    let mut index = length;
    while index < bytes.len() {
        if bytes[index] == b'`' {
            let run = bytes[index..]
                .iter()
                .take_while(|byte| **byte == b'`')
                .count();
            if run == length {
                return bytes[length..index]
                    .iter()
                    .any(|byte| matches!(byte, b'[' | b']'));
            }
            index += run;
        } else {
            index += 1;
        }
    }
    false
}

/// `rest` starts with `<`. Every raw HTML construct and autolink ends with
/// `>`, so without one the `<` is literal. A bracket before the first `>`
/// is treated as hidden (whatever the construct); comments, declarations,
/// CDATA and processing instructions may contain `>` and are always
/// ambiguous. Closing tags and autolinks end at the first `>`. Only an open
/// tag can extend further, through quoted attribute values, so it is parsed
/// with CommonMark's grammar; a tag that does not parse is literal text.
pub(super) fn raw_html_hides_bracket(rest: &str) -> bool {
    let after = &rest[1..];
    let Some(first_close) = after.find('>') else {
        return false;
    };
    if after[..first_close].contains(['[', ']']) {
        return true;
    }
    match after.as_bytes().first() {
        Some(b'!' | b'?') => true,
        Some(byte) if byte.is_ascii_alphabetic() => {
            open_tag_end(after).is_some_and(|end| after[..end].contains(['[', ']']))
        }
        _ => false,
    }
}

/// Byte index of the `>` closing the open tag at the start of `tag` (which
/// starts with the tag name), or `None` when CommonMark would not parse an
/// open tag there. Whitespace is accepted more liberally than CommonMark's
/// "up to one line ending", which can only make more text a tag.
fn open_tag_end(tag: &str) -> Option<usize> {
    let bytes = tag.as_bytes();
    let mut index = 1 + bytes[1..]
        .iter()
        .take_while(|byte| byte.is_ascii_alphanumeric() || **byte == b'-')
        .count();
    loop {
        let before_space = index;
        while bytes.get(index).copied().is_some_and(whitespace) {
            index += 1;
        }
        match *bytes.get(index)? {
            b'>' => return Some(index),
            b'/' => return (bytes.get(index + 1) == Some(&b'>')).then_some(index + 1),
            byte if index > before_space
                && (byte.is_ascii_alphabetic() || matches!(byte, b'_' | b':')) =>
            {
                index += 1 + bytes[index + 1..]
                    .iter()
                    .take_while(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'-')
                    })
                    .count();
                let mut value = index;
                while bytes.get(value).copied().is_some_and(whitespace) {
                    value += 1;
                }
                if bytes.get(value) != Some(&b'=') {
                    continue;
                }
                value += 1;
                while bytes.get(value).copied().is_some_and(whitespace) {
                    value += 1;
                }
                match *bytes.get(value)? {
                    quote @ (b'"' | b'\'') => {
                        let close = tag[value + 1..].find(char::from(quote))?;
                        index = value + 1 + close + 1;
                    }
                    _ => {
                        let length = bytes[value..]
                            .iter()
                            .take_while(|byte| {
                                !whitespace(**byte)
                                    && !matches!(byte, b'"' | b'\'' | b'=' | b'<' | b'>' | b'`')
                            })
                            .count();
                        if length == 0 {
                            return None;
                        }
                        index = value + length;
                    }
                }
            }
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_spans_hide_brackets_only_when_they_contain_one() {
        assert!(code_span_hides_bracket("`]` rest"));
        assert!(code_span_hides_bracket("``a ` [b``"));
        assert!(!code_span_hides_bracket("`a` ] rest"));
        assert!(!code_span_hides_bracket("`` a ` ] no closer"));
    }

    #[test]
    fn rust_generics_are_literal_or_bracket_free_tags() {
        for rest in [
            "<u8>::new()]",
            "<dyn fn()>]",
            "<string, u32>::new()]",
            "< b]",
            "<t as x>]",
        ] {
            assert!(!raw_html_hides_bracket(rest), "{rest:?}");
        }
    }

    #[test]
    fn constructs_that_may_hide_a_bracket_are_reported() {
        for rest in [
            "<b]>",
            "<a title=\"]\">",
            "<a title=\">\" b=]>",
            "<a b='>]'>",
            "<!-- > ] -->",
            "<?x > ] ?>",
            "<http://a]b>",
        ] {
            assert!(raw_html_hides_bracket(rest), "{rest:?}");
        }
        // Invalid tags are literal text.
        assert!(!raw_html_hides_bracket("<a b=\">\" ]"));
        assert!(!raw_html_hides_bracket("<a(b)> ]"));
    }
}
