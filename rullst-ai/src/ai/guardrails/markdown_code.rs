//! Code spans and code blocks for the Markdown image reader.
//!
//! CommonMark never renders an image or reads a link reference definition
//! inside code, so [`blank_code`] blanks recognised code before images and
//! definitions are read. Recognition is deliberately narrower than
//! CommonMark: code it does not recognise stays visible and is judged like
//! any other text, so a miss can only over-block, never hide an image.
//!
//! - A fenced block needs an opening fence at column 0 (or indented by up to
//!   three spaces when the text has no block quote or list item, whose
//!   containers could end the block early) and a matching closing fence. An
//!   unclosed fence is not treated as code.
//! - An indented block needs a preceding blank line (or the start of the
//!   text) and a text without block quotes or list items.
//! - No block is recognised when a line starts an HTML block (`<`), because
//!   the HTML block's own end condition could end it first.
//! - A code span must open and close on one line that is the only line with
//!   a backtick in its blank-line-separated group (so no multi-line pairing
//!   can differ), and only when the text has no `<` that could start raw
//!   HTML or an autolink, which take precedence over code spans.

use std::collections::{HashMap, VecDeque};

/// Spaces before the first other character; tabs are not counted.
fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

/// Indentation in columns with tabs expanded to the next multiple of four.
fn indentation(line: &str) -> usize {
    let mut column = 0;
    for byte in line.bytes() {
        match byte {
            b' ' => column += 1,
            b'\t' => column += 4 - column % 4,
            _ => break,
        }
    }
    column
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// A block quote or list item marker anywhere (read liberally: a false
/// match only disables recognition).
fn container_line(line: &str) -> bool {
    let rest = line.trim_start_matches([' ', '\t']);
    let mut characters = rest.chars();
    match characters.next() {
        Some('>') => true,
        Some('-' | '+' | '*') => characters.next().is_none_or(char::is_whitespace),
        Some(first) if first.is_ascii_digit() => {
            let after_digits = rest.trim_start_matches(|c: char| c.is_ascii_digit());
            let mut after = after_digits.chars();
            matches!(after.next(), Some('.' | ')')) && after.next().is_none_or(char::is_whitespace)
        }
        _ => false,
    }
}

/// An HTML block start: `<` after at most three spaces.
fn html_block_line(line: &str) -> bool {
    leading_spaces(line) <= 3 && line.trim_start_matches(' ').starts_with('<')
}

/// The fence character and length of an opening fence line.
fn opening_fence(line: &str, allow_indent: bool) -> Option<(u8, usize)> {
    let spaces = leading_spaces(line);
    if spaces > 3 || (spaces > 0 && !allow_indent) {
        return None;
    }
    let rest = &line[spaces..];
    let fence = *rest.as_bytes().first()?;
    if fence != b'`' && fence != b'~' {
        return None;
    }
    let length = rest.bytes().take_while(|byte| *byte == fence).count();
    if length < 3 || (fence == b'`' && rest[length..].contains('`')) {
        return None;
    }
    Some((fence, length))
}

/// CommonMark's closing fence: up to three spaces, at least `length` fence
/// characters and nothing but spaces or tabs after them.
fn closing_fence(line: &str, fence: u8, length: usize) -> bool {
    let spaces = leading_spaces(line);
    if spaces > 3 {
        return false;
    }
    let rest = &line[spaces..];
    let run = rest.bytes().take_while(|byte| *byte == fence).count();
    run >= length && rest[run..].trim_matches([' ', '\t', '\n', '\r']).is_empty()
}

/// Marks closed fenced blocks. After an unclosed fence every later line is
/// code for CommonMark, so stopping there can only leave code visible.
fn mark_fenced(lines: &[&str], allow_indent: bool, code: &mut [bool]) {
    let mut index = 0;
    while index < lines.len() {
        let Some((fence, length)) = opening_fence(lines[index], allow_indent) else {
            index += 1;
            continue;
        };
        let Some(offset) = lines[index + 1..]
            .iter()
            .position(|line| closing_fence(line, fence, length))
        else {
            return;
        };
        let close = index + 1 + offset;
        code[index..=close].iter_mut().for_each(|flag| *flag = true);
        index = close + 1;
    }
}

/// Marks indented code: four columns of indentation after a blank line (or
/// the start of the text), continuing through blank and indented lines.
fn mark_indented(lines: &[&str], code: &mut [bool]) {
    let mut previous_blank = true;
    let mut in_block = false;
    for (index, line) in lines.iter().enumerate() {
        if code[index] {
            previous_blank = false;
            in_block = false;
        } else if is_blank(line) {
            previous_blank = true;
        } else if indentation(line) >= 4 && (previous_blank || in_block) {
            code[index] = true;
            in_block = true;
            previous_blank = false;
        } else {
            in_block = false;
            previous_blank = false;
        }
    }
}

/// A `<` that could open raw HTML, a closing tag, a comment, a declaration,
/// a processing instruction or an autolink.
fn may_start_raw_html(text: &str) -> bool {
    text.match_indices('<').any(|(index, _)| {
        text[index + 1..]
            .chars()
            .next()
            .is_some_and(|next| next.is_ascii_alphabetic() || matches!(next, '/' | '!' | '?'))
    })
}

/// Byte ranges of code span contents on one line, paired left to right as
/// CommonMark does. A run after a backslash does not open a span.
fn span_contents(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut runs = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'`' {
            let start = index;
            while index < bytes.len() && bytes[index] == b'`' {
                index += 1;
            }
            let backslashes = bytes[..start]
                .iter()
                .rev()
                .take_while(|byte| **byte == b'\\')
                .count();
            runs.push((start, index, backslashes % 2 == 1));
        } else {
            index += 1;
        }
    }
    let mut by_length: HashMap<usize, VecDeque<usize>> = HashMap::new();
    for (position, (start, end, _)) in runs.iter().enumerate() {
        by_length
            .entry(end - start)
            .or_default()
            .push_back(position);
    }
    let mut contents = Vec::new();
    let mut position = 0;
    while position < runs.len() {
        let (start, end, escaped) = runs[position];
        let candidates = by_length.entry(end - start).or_default();
        while candidates
            .front()
            .is_some_and(|candidate| *candidate <= position)
        {
            candidates.pop_front();
        }
        match candidates.front().copied() {
            Some(closer) if !escaped => {
                contents.push((end, runs[closer].0));
                position = closer + 1;
            }
            _ => position += 1,
        }
    }
    contents
}

fn blank(text: &str, output: &mut String) {
    output.extend(
        text.chars()
            .map(|character| if character == '\n' { '\n' } else { ' ' }),
    );
}

/// Returns `text` with recognised code replaced by spaces. Code span
/// delimiters are kept, so a span inside an image label stays visible to
/// the label reader.
pub(super) fn blank_code(text: &str) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut code = vec![false; lines.len()];
    if !lines.iter().any(|line| html_block_line(line)) {
        let containers = lines.iter().any(|line| container_line(line));
        mark_fenced(&lines, !containers, &mut code);
        if !containers {
            mark_indented(&lines, &mut code);
        }
    }
    let spans = !may_start_raw_html(text);
    let mut output = String::with_capacity(text.len());
    let mut group_start = 0;
    while group_start < lines.len() {
        // A group is a run of visible non-blank lines; blocks and blank
        // lines are copied or blanked one by one.
        if code[group_start] || is_blank(lines[group_start]) {
            if code[group_start] {
                blank(lines[group_start], &mut output);
            } else {
                output.push_str(lines[group_start]);
            }
            group_start += 1;
            continue;
        }
        let group_end = (group_start..lines.len())
            .find(|&index| code[index] || is_blank(lines[index]))
            .unwrap_or(lines.len());
        let group = &lines[group_start..group_end];
        let with_backticks: Vec<usize> = (0..group.len())
            .filter(|&index| group[index].contains('`'))
            .collect();
        for (index, line) in group.iter().enumerate() {
            if spans && with_backticks == [index] {
                let mut copied = 0;
                for (start, end) in span_contents(line) {
                    output.push_str(&line[copied..start]);
                    blank(&line[start..end], &mut output);
                    copied = end;
                }
                output.push_str(&line[copied..]);
            } else {
                output.push_str(line);
            }
        }
        group_start = group_end;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_fences_and_indented_blocks_are_blanked() {
        let image = "![a](//evil.example/x)";
        let text = format!("intro\n```md\n{image}\n```\nafter\n");
        let blanked = format!(
            "intro\n{}\n{}\n{}\nafter\n",
            " ".repeat(5),
            " ".repeat(image.len()),
            " ".repeat(3)
        );
        assert_eq!(blank_code(&text), blanked);
        let tilde = "~~~~\n[r]: //evil.example/x\n~~~~~\n";
        assert!(!blank_code(tilde).contains("evil"));
        let indented = "text\n\n    ![a](//evil.example/x)\n\nmore";
        assert!(!blank_code(indented).contains("evil"));
    }

    #[test]
    fn ambiguous_layouts_leave_code_visible() {
        for text in [
            // Unclosed fence.
            "```\n![a](//evil.example/x)\n",
            // A four-space line is content, not a closer; the real closer
            // follows, and the image after it renders.
            "```\n    ```\n```\n![a](//evil.example/x)\n",
            // An HTML block could end before the fence closes.
            "<div>\n```\n\n![a](//evil.example/x)\n```\n",
            // A list item could end an indented fence early.
            "- item\n  ```\n![a](//evil.example/x)\n  ```\n",
            // Indented lines inside a list item are paragraphs.
            "- item\n\n    ![a](//evil.example/x)\n",
            // Indented code cannot interrupt a paragraph.
            "para\n    ![a](//evil.example/x)\n",
            // An info string with a backtick is not a fence.
            "``` `x`\n![a](//evil.example/x)\n```\n",
        ] {
            assert!(blank_code(text).contains("evil"), "{text:?}");
        }
    }

    #[test]
    fn code_spans_pair_left_to_right_on_unambiguous_lines() {
        let spaces = |count: usize| " ".repeat(count);
        assert_eq!(
            blank_code("a `![x](//e.x/y)` b"),
            format!("a `{}` b", spaces(13))
        );
        assert_eq!(blank_code("``a ` b`` c"), format!("``{}`` c", spaces(5)));
        // Unequal runs do not pair; an escaped run does not open.
        assert_eq!(blank_code("``a` b"), "``a` b");
        assert_eq!(blank_code("\\`a` b`"), format!("\\`a`{}`", spaces(2)));
        for text in [
            // Multi-line pairing depends on the block structure.
            "`a\n# ![x](//e.x/y) `",
            // Raw HTML takes precedence over a code span.
            "<a title=\"`\">![x](//e.x/y)<b title=\"`\">",
        ] {
            assert!(blank_code(text).contains("![x](//e.x/y)"), "{text:?}");
        }
    }
}
