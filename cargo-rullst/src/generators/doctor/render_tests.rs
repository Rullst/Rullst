//! The doctor's text: aligned check lines, fixes, hints, docs and a summary.
#![allow(clippy::unwrap_used)]

use super::*;
use crate::generators::doctor::report::{Check, Group};

fn mixed() -> Report {
    Report::new(vec![
        Group::new(
            "toolchain",
            "Toolchain",
            "toolchain",
            vec![
                Check::pass("a.pass", "Rust", "1.96.0"),
                Check::warn("a.warn", "Linker", "default", "install mold\nthen retry"),
                Check::info("a.info", "Wasm", "not installed").with_fix("rustup target add"),
            ],
        ),
        Group::new(
            "disk",
            "Disk",
            "disk",
            vec![Check::fail("b.fail", "Free space", "0.5 GiB", "free space")],
        ),
    ])
}

const DOCS: &str = "https://rullst.github.io/Rullst/book/cli_reference.html#doctor-";

#[test]
fn plain_text_aligns_checks_and_lists_fixes_hints_and_docs() {
    let expected = format!(
        "Toolchain
  ✓ Rust        1.96.0
  ! Linker      default
      fix   install mold
            then retry
      docs  {DOCS}toolchain
  · Wasm        not installed
      hint  rustup target add

Disk
  ✗ Free space  0.5 GiB
      fix   free space
      docs  {DOCS}disk

Summary  1 passed · 1 warning · 1 failed · 1 info
Fix the ✗ items above, then run `cargo rullst doctor` again.
"
    );
    assert_eq!(render(&mixed(), Style::PLAIN), expected);
    assert_eq!(
        header(Style::PLAIN),
        format!("Rullst doctor · v{}\n\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn warnings_alone_pass_with_a_note_and_counts_are_plural() {
    let warned = Report::new(vec![Group::new(
        "project",
        "Project",
        "project",
        vec![
            Check::warn("w.1", "One", "a", "fix a"),
            Check::warn("w.2", "Two", "b", "fix b"),
        ],
    )]);
    let text = render(&warned, Style::PLAIN);
    assert!(text.ends_with(
        "Summary  0 passed · 2 warnings · 0 failed · 0 info
Warnings do not fail the doctor; the fix lines above resolve them.
"
    ));
    let clean = Report::new(vec![Group::new(
        "project",
        "Project",
        "project",
        vec![Check::pass("p.1", "One", "ok")],
    )]);
    let text = render(&clean, Style::PLAIN);
    assert!(text.ends_with("Summary  1 passed · 0 warnings · 0 failed · 0 info\n"));
}

#[test]
fn colour_marks_statuses_and_mutes_zero_counts() {
    let colour = Style::TRUE_COLOR;
    let text = render(&mixed(), colour);
    for (glyph, color) in [
        ("✓", style::PASS),
        ("!", style::WARN),
        ("✗", style::FAIL),
        ("·", style::MUTED),
    ] {
        assert!(text.contains(&colour.bold(glyph, color)), "{glyph}");
    }
    assert!(text.contains(&colour.bold("1 warning", style::WARN)));

    let clean = Report::new(vec![Group::new(
        "project",
        "Project",
        "project",
        vec![Check::pass("p.1", "One", "ok")],
    )]);
    let text = render(&clean, colour);
    assert!(text.contains(&colour.bold("1 passed", style::PASS)));
    assert!(text.contains(&colour.paint("0 warnings", style::MUTED)));
}
