//! The `rullst-upgrade: reviewed <CODE>` marker. A comment naming a REVIEW
//! rule after the code on a finding's line, or alone on the line directly
//! above it, records that the location was reviewed: the plan leaves that
//! finding out and reports the next unmarked location of the rule in the
//! file, if any. Any comment syntax works (`//`, `#`, `<!-- -->`, `--`),
//! several codes may follow one marker (`V13-A, V13-B`), and words after the
//! codes are free text. MUST-CHANGE findings ignore the marker: their code
//! does not build until it changes.

use super::{FindingKind, Rule, SourceFinding};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The text that starts a marker.
pub(crate) const MARKER: &str = "rullst-upgrade: reviewed";
/// Larger files are not read again for markers.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Whether `word` has the shape of a rule code, such as `V13-PAGE-BOUNDS`.
fn is_code(word: &str) -> bool {
    word.len() > 4
        && word.starts_with("V13-")
        && word
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
}

/// The rule codes the marker on `line` names, in order.
pub(super) fn marked_codes(line: &str) -> Vec<&str> {
    let Some((_, rest)) = line.split_once(MARKER) else {
        return Vec::new();
    };
    rest.split(|character: char| character.is_whitespace() || character == ',')
        .filter(|word| !word.is_empty())
        .map(|word| word.trim_end_matches(|character: char| !character.is_ascii_alphanumeric()))
        .take_while(|word| is_code(word))
        .collect()
}

/// Whether the marker on `line` is the only thing on it, after comment
/// punctuation such as `//`, `#` or `<!--`.
fn alone(line: &str) -> bool {
    line.split_once(MARKER)
        .is_some_and(|(before, _)| !before.chars().any(char::is_alphanumeric))
}

/// Whether the 1-based `line` of `lines` carries a marker for `rule`, or the
/// line above it holds only such a marker. Only REVIEW rules can be marked.
pub(super) fn covers(lines: &[&str], line: usize, rule: &Rule) -> bool {
    if rule.kind != FindingKind::Review {
        return false;
    }
    let marks = |text: &&str| marked_codes(text).contains(&rule.code);
    let own = line.checked_sub(1).and_then(|index| lines.get(index));
    let above = line.checked_sub(2).and_then(|index| lines.get(index));
    own.is_some_and(marks) || above.filter(|text| alone(text)).is_some_and(marks)
}

/// The first line of `lines` satisfying `matches` that no marker covers for
/// `rule`, and the rule once for every covered matching line.
pub(super) fn first_unreviewed(
    lines: &[&str],
    rule: &'static Rule,
    matches: impl Fn(&str) -> bool,
    reviewed: &mut Vec<&'static Rule>,
) -> Option<usize> {
    let mut first = None;
    for (index, line) in lines.iter().enumerate() {
        if !matches(line.trim()) {
            continue;
        }
        if covers(lines, index + 1, rule) {
            reviewed.push(rule);
        } else if first.is_none() {
            first = Some(index + 1);
        }
    }
    first
}

/// A regular, bounded UTF-8 file; `None` for anything else.
fn read(path: &Path) -> Option<String> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// Removes the findings a marker covers and returns their rules. Each file
/// is read once.
pub(super) fn drop_reviewed(findings: &mut Vec<SourceFinding>) -> Vec<&'static Rule> {
    let mut texts: BTreeMap<PathBuf, Option<String>> = BTreeMap::new();
    let mut reviewed = Vec::new();
    findings.retain(|finding| {
        if finding.rule.kind != FindingKind::Review {
            return true;
        }
        let text = texts
            .entry(finding.path.clone())
            .or_insert_with(|| read(&finding.path));
        let lines: Vec<&str> = text.as_deref().unwrap_or_default().lines().collect();
        let covered = covers(&lines, finding.line, finding.rule);
        if covered {
            reviewed.push(finding.rule);
        }
        !covered
    });
    reviewed
}
