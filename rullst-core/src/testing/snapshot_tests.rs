use super::snapshot::{Outcome, check_snapshot, line_diff, snapshot_path, update_requested};
use super::{SnapshotOptions, normalize_html};
use std::path::Path;

const PLAIN: SnapshotOptions = SnapshotOptions::new();

#[test]
fn whitespace_between_tags_is_insignificant_and_tags_get_their_own_lines() {
    let compact = r#"<div class="card"><h1>Title</h1><p>Hello <b>you</b>!</p></div>"#;
    let indented =
        "<div   class=\"card\" >\r\n  <h1>Title</h1>\n\n  <p>Hello <b>you</b>!</p>\n</div>\n";
    let expected = "<div class=\"card\">\n<h1>Title</h1>\n<p>Hello <b>you</b>!</p>\n</div>\n";
    assert_eq!(normalize_html(compact, PLAIN), expected);
    assert_eq!(normalize_html(indented, PLAIN), expected);
    assert_eq!(normalize_html(expected, PLAIN), expected, "idempotent");
}

#[test]
fn text_whitespace_collapses_but_raw_text_and_quoted_values_are_kept() {
    let html = "<p>a   b\n c</p><pre>  keep\n    this  </pre><textarea>\n x  </textarea>\
                <script>if (a < b) {\n  go();\n}</script><span title=\"two  spaces\">&nbsp;</span>";
    let normalized = normalize_html(html, PLAIN);
    assert_eq!(
        normalized,
        "<p>a b c</p>\n<pre>  keep\n    this  </pre>\n<textarea>\n x  </textarea>\n\
         <script>if (a < b) {\n  go();\n}</script>\n<span title=\"two  spaces\">&nbsp;</span>\n"
    );
    assert_eq!(normalize_html(&normalized, PLAIN), normalized);
    assert_eq!(
        normalize_html("a < b and <!-- note -->", PLAIN),
        "a < b and <!-- note -->\n"
    );
    assert_eq!(normalize_html(" \n ", PLAIN), "\n");
}

#[test]
fn nonces_and_csrf_tokens_are_masked_only_when_asked() {
    let page = concat!(
        r#"<meta http-equiv="Content-Security-Policy" content="script-src 'self' 'nonce-QWxhZGRpbg=='">"#,
        r#"<script nonce="QWxhZGRpbg==" src="/app.js"></script><style NONCE='abc'>p{}</style>"#,
        r#"<meta name="csrf-token" content="t0k3n">"#,
        r#"<form><input type="hidden" name="_token" value="t0k3n"><input name="email" value="kept"></form>"#,
        r#"<div hx-headers="{&quot;X-CSRF-Token&quot;: &quot;t0k3n&quot;}"></div>"#,
        r#"<div hx-headers='{"x-csrf-token":"t0k3n"}'></div>"#,
    );
    let plain = normalize_html(page, PLAIN);
    assert!(plain.contains("t0k3n") && plain.contains("QWxhZGRpbg=="));

    let masked = normalize_html(page, PLAIN.mask_nonce().mask_csrf_token());
    assert!(!masked.contains("t0k3n"), "{masked}");
    assert!(!masked.contains("QWxhZGRpbg=="), "{masked}");
    assert!(!masked.contains("abc"), "{masked}");
    for expected in [
        "'nonce-{NONCE}'",
        r#"<script nonce="{NONCE}" src="/app.js">"#,
        "<style NONCE='{NONCE}'>",
        r#"<meta name="csrf-token" content="{CSRF_TOKEN}">"#,
        r#"<input type="hidden" name="_token" value="{CSRF_TOKEN}">"#,
        r#"<input name="email" value="kept">"#,
        "&quot;X-CSRF-Token&quot;: &quot;{CSRF_TOKEN}&quot;",
        r#""x-csrf-token":"{CSRF_TOKEN}""#,
    ] {
        assert!(masked.contains(expected), "missing {expected} in\n{masked}");
    }

    let nonce_only = normalize_html(page, PLAIN.mask_nonce());
    assert!(nonce_only.contains("t0k3n") && !nonce_only.contains("QWxhZGRpbg=="));
    assert_eq!(
        normalize_html(&masked, PLAIN.mask_nonce().mask_csrf_token()),
        masked
    );
}

#[test]
fn snapshot_names_cannot_leave_the_snapshot_folder() {
    let root = Path::new("/crate");
    assert_eq!(
        snapshot_path(root, "pages/home").unwrap(),
        Path::new("/crate/tests/snapshots/pages/home.html")
    );
    assert_eq!(
        snapshot_path(root, "v1.2_page-x").unwrap(),
        Path::new("/crate/tests/snapshots/v1.2_page-x.html")
    );
    for invalid in [
        "",
        "../escape",
        "a//b",
        "/abs",
        "a/./b",
        "sp ace",
        "back\\slash",
        "é",
    ] {
        assert!(snapshot_path(root, invalid).is_err(), "{invalid:?}");
    }
}

#[test]
fn only_explicit_values_request_an_update() {
    for value in ["1", "true", "YES", " 1 "] {
        assert!(update_requested(Some(value)), "{value}");
    }
    for value in ["", "0", "false", "no", "2"] {
        assert!(!update_requested(Some(value)), "{value}");
    }
    assert!(!update_requested(None));
}

#[test]
fn missing_and_changed_snapshots_fail_unless_updating() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tests/snapshots/nested/page.html");
    let first = normalize_html("<main><h1>One</h1></main>", PLAIN);

    let missing = check_snapshot(&path, "nested/page", &first, PLAIN, false).unwrap_err();
    assert!(
        missing.contains("HTML snapshot `nested/page` is missing"),
        "{missing}"
    );
    assert!(missing.contains("RULLST_UPDATE_SNAPSHOTS=1"), "{missing}");
    assert!(!path.exists());

    assert_eq!(
        check_snapshot(&path, "nested/page", &first, PLAIN, true),
        Ok(Outcome::Written)
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), first);
    assert_eq!(
        check_snapshot(&path, "nested/page", &first, PLAIN, false),
        Ok(Outcome::Matched)
    );

    // A hand-indented snapshot still matches: both sides are normalised.
    std::fs::write(&path, "<main>\n    <h1>One</h1>\n</main>").unwrap();
    assert_eq!(
        check_snapshot(&path, "nested/page", &first, PLAIN, false),
        Ok(Outcome::Matched)
    );

    let second = normalize_html("<main><h1>Two</h1></main>", PLAIN);
    let changed = check_snapshot(&path, "nested/page", &second, PLAIN, false).unwrap_err();
    assert!(changed.contains("does not match"), "{changed}");
    assert!(changed.contains("-   2 | <h1>One</h1>"), "{changed}");
    assert!(changed.contains("+   2 | <h1>Two</h1>"), "{changed}");
    assert!(
        !changed.contains("<main>\n"),
        "unchanged lines are omitted: {changed}"
    );

    assert_eq!(
        check_snapshot(&path, "nested/page", &second, PLAIN, true),
        Ok(Outcome::Written)
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), second);
}

#[test]
fn long_diffs_are_capped() {
    let old: String = (0..60).map(|n| format!("<p>{n}</p>\n")).collect();
    let new: String = (0..60).map(|n| format!("<p>{}</p>\n", n + 100)).collect();
    let diff = line_diff(&old, &new);
    assert!(diff.contains("… 80 more changed lines"), "{diff}");
    assert_eq!(diff.lines().count(), 42);
}

#[test]
fn the_macro_compares_with_this_crates_snapshot_folder() {
    crate::assert_html_snapshot!(
        "testing_helper_example",
        r#"<section class="hero">
             <h1>Snapshot</h1>
             <form><input type="hidden" name="_token" value="per-request"></form>
           </section>"#,
        SnapshotOptions::new().mask_csrf_token()
    );
}

#[test]
#[should_panic(expected = "invalid HTML snapshot name")]
fn the_macro_rejects_an_escaping_name() {
    crate::assert_html_snapshot!("../outside", "<p>x</p>");
}

/// Reference diff: an independent longest-common-subsequence walk that, like
/// `line_diff`, prefers a deletion to an insertion and shows 40 lines.
fn reference_diff(expected: &str, actual: &str) -> String {
    let old: Vec<&str> = expected.lines().collect();
    let new: Vec<&str> = actual.lines().collect();
    // lcs[i][j]: longest common subsequence of old[i..] and new[j..].
    let mut lcs = vec![vec![0_usize; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            lcs[i][j] = if old[i] == new[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let mut changes = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < old.len() || j < new.len() {
        if i < old.len() && j < new.len() && old[i] == new[j] {
            i += 1;
            j += 1;
        } else if j == new.len() || (i < old.len() && lcs[i + 1][j] >= lcs[i][j + 1]) {
            changes.push(format!("  -{:>4} | {}\n", i + 1, old[i]));
            i += 1;
        } else {
            changes.push(format!("  +{:>4} | {}\n", j + 1, new[j]));
            j += 1;
        }
    }
    let mut out = String::from("  (- snapshot, + rendered)\n");
    for change in changes.iter().take(40) {
        out.push_str(change);
    }
    if changes.len() > 40 {
        out.push_str(&format!("  … {} more changed lines\n", changes.len() - 40));
    }
    out
}

#[test]
fn line_diffs_match_a_reference_longest_common_subsequence() {
    let alphabet = ["<p>a</p>", "<p>b</p>", "<p>c</p>", ""];
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = |bound: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % bound) as usize
    };
    for _ in 0..3_000 {
        let mut text = || {
            let lines = next(8);
            (0..lines)
                .map(|_| alphabet[next(4)])
                .collect::<Vec<_>>()
                .join("\n")
        };
        let (old, new) = (text(), text());
        assert_eq!(
            line_diff(&old, &new),
            reference_diff(&old, &new),
            "{old:?} -> {new:?}"
        );
    }
    assert_eq!(
        line_diff("a\nb\nc\nd", "b\nc\nd\na"),
        "  (- snapshot, + rendered)\n  -   1 | a\n  +   4 | a\n"
    );
}

#[test]
fn forty_changed_lines_are_shown_without_a_remainder() {
    let old: String = (0..40).map(|n| format!("<p>{n}</p>\n")).collect();
    let diff = line_diff(&old, "");
    assert_eq!(diff.lines().count(), 41);
    assert!(!diff.contains("more changed lines"), "{diff}");
}

#[test]
fn diffs_over_four_million_cells_show_only_the_first_difference() {
    let text = |prefix: &str, lines: usize| {
        let mut text = String::from("same\nsame\n");
        for n in 2..lines {
            text.push_str(&format!("{prefix}{n}\n"));
        }
        text
    };
    // 1,000 by 4,001 and 4,001 by 1,000 cells, just over the limit.
    for (old, new) in [(999, 4_000), (4_000, 999)] {
        assert_eq!(
            line_diff(&text("o", old), &text("n", new)),
            "  (- snapshot, + rendered)\n  -   3 | o2\n  +   3 | n2\n",
            "{old} by {new}"
        );
    }
}

#[test]
fn an_unreadable_snapshot_is_an_error_not_a_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let error = check_snapshot(dir.path(), "folder", "<p>x</p>\n", PLAIN, false).unwrap_err();
    assert!(error.contains("could not read HTML snapshot"), "{error}");
}
