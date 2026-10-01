//! A small, bounded line diff for reviewing proposed file changes.
//!
//! Common leading and trailing lines are trimmed first; the remaining middle
//! uses a longest-common-subsequence table when it fits a fixed cell budget,
//! and otherwise is shown as one replaced block. Every rendered line is
//! sanitised, so file content cannot emit terminal controls.

use super::term::{Style, sanitize};

/// Largest LCS table (old lines × new lines) computed for one change.
const MAX_CELLS: usize = 1_000_000;
/// Unchanged lines shown around each change.
const CONTEXT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Keep,
    Remove,
    Add,
}

/// Line operations transforming `old` into `new`.
fn operations<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(Op, &'a str)> {
    let prefix = old
        .iter()
        .zip(new)
        .take_while(|(left, right)| left == right)
        .count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    let old_middle = &old[prefix..old.len() - suffix];
    let new_middle = &new[prefix..new.len() - suffix];
    let mut ops: Vec<(Op, &str)> = old[..prefix].iter().map(|line| (Op::Keep, *line)).collect();
    let cells = (old_middle.len() + 1).saturating_mul(new_middle.len() + 1);
    if cells <= MAX_CELLS {
        ops.extend(lcs(old_middle, new_middle));
    } else {
        ops.extend(old_middle.iter().map(|line| (Op::Remove, *line)));
        ops.extend(new_middle.iter().map(|line| (Op::Add, *line)));
    }
    ops.extend(
        old[old.len() - suffix..]
            .iter()
            .map(|line| (Op::Keep, *line)),
    );
    ops
}

fn lcs<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(Op, &'a str)> {
    let width = new.len() + 1;
    let mut table = vec![0u32; (old.len() + 1) * width];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            table[i * width + j] = if old[i] == new[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut ops = Vec::with_capacity(old.len() + new.len());
    while i < old.len() && j < new.len() {
        if old[i] == new[j] {
            ops.push((Op::Keep, old[i]));
            i += 1;
            j += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            ops.push((Op::Remove, old[i]));
            i += 1;
        } else {
            ops.push((Op::Add, new[j]));
            j += 1;
        }
    }
    ops.extend(old[i..].iter().map(|line| (Op::Remove, *line)));
    ops.extend(new[j..].iter().map(|line| (Op::Add, *line)));
    ops
}

/// Counts of added and removed lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Stats {
    pub added: usize,
    pub removed: usize,
}

/// Renders a unified-style diff with hunk headers and context lines.
pub(super) fn render(old: &str, new: &str, style: Style) -> (String, Stats) {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let ops = operations(&old_lines, &new_lines);
    let mut stats = Stats::default();
    for (op, _) in &ops {
        match op {
            Op::Add => stats.added += 1,
            Op::Remove => stats.removed += 1,
            Op::Keep => {}
        }
    }
    // Mark which operations are visible: changes plus surrounding context.
    let mut visible = vec![false; ops.len()];
    for (index, (op, _)) in ops.iter().enumerate() {
        if *op != Op::Keep {
            let start = index.saturating_sub(CONTEXT);
            let end = (index + CONTEXT + 1).min(ops.len());
            visible[start..end].iter_mut().for_each(|flag| *flag = true);
        }
    }
    let mut output = String::new();
    let (mut old_line, mut new_line) = (1usize, 1usize);
    let mut index = 0;
    while index < ops.len() {
        if !visible[index] {
            if ops[index].0 != Op::Add {
                old_line += 1;
            }
            if ops[index].0 != Op::Remove {
                new_line += 1;
            }
            index += 1;
            continue;
        }
        let end = (index..ops.len())
            .find(|&i| !visible[i])
            .unwrap_or(ops.len());
        let hunk = &ops[index..end];
        let old_count = hunk.iter().filter(|(op, _)| *op != Op::Add).count();
        let new_count = hunk.iter().filter(|(op, _)| *op != Op::Remove).count();
        output.push_str(&style.cyan(&format!(
            "@@ -{},{} +{},{} @@",
            if old_count == 0 {
                old_line - 1
            } else {
                old_line
            },
            old_count,
            if new_count == 0 {
                new_line - 1
            } else {
                new_line
            },
            new_count
        )));
        output.push('\n');
        for (op, line) in hunk {
            let line = sanitize(line);
            let rendered = match op {
                Op::Keep => style.dim(&format!("  {line}")),
                Op::Remove => style.red(&format!("- {line}")),
                Op::Add => style.green(&format!("+ {line}")),
            };
            output.push_str(&rendered);
            output.push('\n');
        }
        old_line += old_count;
        new_line += new_count;
        index = end;
    }
    if old != new && old.lines().eq(new.lines()) {
        output.push_str(&style.dim("  (only line endings or the final newline change)"));
        output.push('\n');
    }
    (output, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: Style = Style { color: false };

    #[test]
    fn edits_show_context_and_changed_lines() {
        let old = "a\nb\nc\nd\ne\nf\ng\nh\ni\n";
        let new = "a\nb\nc\nd\nE\nf\ng\nh\ni\n";
        let (diff, stats) = render(old, new, PLAIN);
        assert_eq!(
            stats,
            Stats {
                added: 1,
                removed: 1
            }
        );
        assert_eq!(
            diff,
            "@@ -2,7 +2,7 @@\n  b\n  c\n  d\n- e\n+ E\n  f\n  g\n  h\n"
        );
    }

    #[test]
    fn new_files_are_all_additions() {
        let (diff, stats) = render("", "one\ntwo\n", PLAIN);
        assert_eq!(
            stats,
            Stats {
                added: 2,
                removed: 0
            }
        );
        assert_eq!(diff, "@@ -0,0 +1,2 @@\n+ one\n+ two\n");
    }

    #[test]
    fn separate_changes_get_separate_hunks() {
        let old: String = (1..=20).map(|n| format!("{n}\n")).collect();
        let new: String = (1..=20)
            .map(|n| match n {
                2 => "two\n".to_string(),
                19 => "nineteen\n".to_string(),
                n => format!("{n}\n"),
            })
            .collect();
        let (diff, stats) = render(&old, &new, PLAIN);
        assert_eq!(diff.matches("@@ -").count(), 2);
        assert_eq!(
            stats,
            Stats {
                added: 2,
                removed: 2
            }
        );
    }

    #[test]
    fn large_rewrites_fall_back_to_a_replaced_block() {
        let old: String = (0..1500).map(|n| format!("old {n}\n")).collect();
        let new: String = (0..1500).map(|n| format!("new {n}\n")).collect();
        let (_, stats) = render(&old, &new, PLAIN);
        assert_eq!(
            stats,
            Stats {
                added: 1500,
                removed: 1500
            }
        );
    }

    #[test]
    fn hostile_content_is_escaped_in_the_diff() {
        let (diff, _) = render("", "x\x1b[2Jy\n", Style { color: true });
        assert!(!diff.contains("\x1b[2J"));
        assert!(diff.contains("\\u{1b}[2J"));
    }

    #[test]
    fn line_ending_only_changes_are_reported() {
        let (diff, stats) = render("a\n", "a", PLAIN);
        assert_eq!(stats, Stats::default());
        assert!(diff.contains("line endings"));
    }
}
