//! Numbers, scores and notes as the pages write them.

use rullst::html;
use rullst::html::Markup;

use super::{Cell, FLAGS};
use crate::pages::Lang;

/// `value` with `decimals` decimals and thousands separators: `1,045.5` in English,
/// `1.045,5` in Portuguese.
pub fn number(value: f64, decimals: usize, lang: Lang) -> String {
    let fixed = format!("{value:.decimals$}");
    let (integer, fraction) = fixed.split_once('.').unwrap_or((fixed.as_str(), ""));
    let (sign, digits) = integer
        .strip_prefix('-')
        .map_or(("", integer), |digits| ("-", digits));
    let (group, point) = if lang.is_pt() { ('.', ',') } else { (',', '.') };
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(group);
        }
        grouped.push(digit);
    }
    if fraction.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped}{point}{fraction}")
    }
}

/// Whole MiB.
pub fn mib(value: f64, lang: Lang) -> String {
    number(value, 0, lang)
}

/// A score as the pages write it (`0.50×`), with its flag signs when `signs` is true
/// (`*` right after the score, the other signs after a space each).
pub fn score(cell: &Cell, lang: Lang, signs: bool) -> String {
    let value = cell.score.unwrap_or_default();
    let mut text = number(value, if value >= 0.1 { 2 } else { 3 }, lang) + "×";
    if !signs {
        return text;
    }
    if cell.has("cpu_cost") {
        text.push('*');
    }
    for (flag, sign) in FLAGS {
        if flag != "cpu_cost" && cell.has(flag) {
            text.push(' ');
            text.push_str(sign);
        }
    }
    text
}

/// A backtick-quoted span that becomes `<code>`: 1 to 80 characters on one line, not all
/// blank and without `<` or `>`.
fn code_span(piece: &str) -> bool {
    let length = piece.chars().count();
    (1..=80).contains(&length)
        && !piece.trim().is_empty()
        && !piece.contains(['<', '>', '\r', '\n'])
}

/// A note's text, escaped, with its short backtick spans as `<code>`. Backticks pair up in
/// order; any other pair and a final unpaired backtick stay literal text.
pub fn render_note(note: &str) -> Markup {
    let pieces: Vec<&str> = note.split('`').collect();
    pieces
        .iter()
        .enumerate()
        .map(|(index, piece)| {
            if index % 2 == 0 {
                return html! { {*piece} };
            }
            let closed = index + 1 < pieces.len();
            if closed && code_span(piece) {
                html! { <code>{*piece}</code> }
            } else {
                let literal = format!("`{piece}{}", if closed { "`" } else { "" });
                html! { {literal} }
            }
        })
        .collect()
}

/// Whether Rullst `version` honours `RULLST_ACCESS_LOG=off`, which Rullst/Benchmarks sets:
/// 13.0.0-alpha.6 and every later release do; earlier ones wrote an access-log line per request.
pub fn access_log_off(version: &str) -> bool {
    let (release, pre) = version.split_once('-').unwrap_or((version, ""));
    let parts: Vec<Option<u64>> = release.split('.').map(|part| part.parse().ok()).collect();
    let [Some(major), Some(minor), Some(patch)] = parts[..] else {
        return false;
    };
    if (major, minor, patch) != (13, 0, 0) {
        return (major, minor, patch) > (13, 0, 0);
    }
    match pre.strip_prefix("alpha.") {
        Some(number) => number.parse::<u64>().is_ok_and(|number| number >= 6),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{access_log_off, number, render_note};

    #[test]
    fn the_access_log_is_off_from_alpha_6() {
        for version in [
            "13.0.0-alpha.6",
            "13.0.0-rc.1",
            "13.0.0",
            "13.1.0",
            "14.0.0-alpha.1",
        ] {
            assert!(access_log_off(version), "{version}");
        }
        for version in [
            "13.0.0-alpha.5",
            "13.0.0-alpha.1",
            "12.3.0",
            "13.0",
            "unknown",
        ] {
            assert!(!access_log_off(version), "{version}");
        }
    }
    use crate::pages::Lang;

    #[test]
    fn numbers_round_like_python_and_group_by_language() {
        // Python's f"{v:,.2f}": correctly rounded, ties to even on the binary value.
        assert_eq!(number(0.125, 2, Lang::En), "0.12");
        assert_eq!(number(0.375, 2, Lang::En), "0.38");
        assert_eq!(number(2.5, 0, Lang::En), "2");
        assert_eq!(number(3.5, 0, Lang::En), "4");
        assert_eq!(number(1045.0, 0, Lang::En), "1,045");
        assert_eq!(number(1045.0, 0, Lang::PtBr), "1.045");
        assert_eq!(number(1234567.891, 1, Lang::PtBr), "1.234.567,9");
        assert_eq!(number(5.48, 1, Lang::En), "5.5");
        assert_eq!(number(0.07, 3, Lang::PtBr), "0,070");
    }

    #[test]
    fn notes_turn_only_short_balanced_spans_into_code() {
        for (note, html) in [
            ("`a` & `b`", "<code>a</code> &amp; <code>b</code>"),
            ("`a", "`a"),
            ("``", "``"),
            ("` `", "` `"),
            ("`x\ny`", "`x\ny`"),
            ("`\"q\"`", "<code>&quot;q&quot;</code>"),
            ("`<i>` `ok` `", "`&lt;i&gt;` <code>ok</code> `"),
        ] {
            assert_eq!(render_note(note).as_str(), html, "{note:?}");
        }
        let long = format!("`{}`", "x".repeat(81));
        assert_eq!(render_note(&long).as_str(), long);
    }
}
