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
