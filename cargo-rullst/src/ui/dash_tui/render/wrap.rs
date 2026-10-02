//! Log wrapping done before rendering, so a pane's scroll offset counts
//! exactly the rows that are drawn and the newest line stays visible.

/// Greedy word wrap to `width` characters; a word longer than a line is
/// split. Always returns at least one (possibly empty) line.
pub(super) fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut length = 0;
    for word in text.split(' ') {
        let word_length = word.chars().count();
        if length > 0 && length + 1 + word_length > width {
            lines.push(std::mem::take(&mut current));
            length = 0;
        }
        if length > 0 {
            current.push(' ');
            length += 1;
        }
        let mut rest = word;
        while length + rest.chars().count() > width {
            let split = rest
                .char_indices()
                .nth(width - length)
                .map_or(rest.len(), |(index, _)| index);
            current.push_str(&rest[..split]);
            lines.push(std::mem::take(&mut current));
            length = 0;
            rest = &rest[split..];
        }
        current.push_str(rest);
        length += rest.chars().count();
    }
    lines.push(current);
    lines
}

#[cfg(test)]
mod tests {
    use super::wrap_text;

    #[test]
    fn words_wrap_greedily_and_long_words_are_split() {
        assert_eq!(wrap_text("", 5), [""]);
        assert_eq!(wrap_text("ready", 5), ["ready"]);
        assert_eq!(wrap_text("app is ready now", 6), ["app is", "ready", "now"]);
        assert_eq!(
            wrap_text("/_rullst/dev-telemetry.", 10),
            ["/_rullst/d", "ev-telemet", "ry."]
        );
        assert_eq!(
            wrap_text("at 127.0.0.1:3000 ok", 8),
            ["at", "127.0.0.", "1:3000", "ok"]
        );
        assert_eq!(wrap_text("açúcar é bom", 6), ["açúcar", "é bom"]);
        assert_eq!(wrap_text("x", 0), ["x"]);
        for line in wrap_text(&"word ".repeat(40), 7) {
            assert!(line.chars().count() <= 7, "{line:?}");
        }
    }
}
