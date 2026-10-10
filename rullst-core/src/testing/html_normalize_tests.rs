//! Direct checks of the tokenizer helpers behind `normalize_html`.

use super::*;

fn names(tag: &str) -> Vec<&str> {
    attributes(tag)
        .into_iter()
        .map(|attribute| attribute.name)
        .collect()
}

#[test]
fn a_quoted_closing_bracket_does_not_end_a_tag() {
    assert_eq!(tag_end(r#"<a title="x>y">rest"#), 15);
    assert_eq!(tag_end("<a title='x>y'>rest"), 15);
    assert_eq!(tag_end("<a href=x>rest"), 10);
    assert_eq!(tag_end("<a title=\"open"), 14);
}

#[test]
fn only_a_tag_opener_ends_a_text_run() {
    assert_eq!(next_tag("a < b <p>"), 6);
    assert_eq!(next_tag("a <!-- c -->"), 2);
    assert_eq!(next_tag("plain"), 5);
}

#[test]
fn a_raw_text_element_closes_only_at_its_own_name() {
    assert_eq!(closing_tag("a</SCRIPT>", "script"), 1);
    assert_eq!(closing_tag("a</script", "script"), 1);
    assert_eq!(closing_tag("</script\t>", "script"), 0);
    assert_eq!(closing_tag("</script/>", "script"), 0);
    assert_eq!(closing_tag("ab</scripts></script>", "script"), 12);
    assert_eq!(closing_tag("no close", "script"), 8);
}

#[test]
fn attributes_skip_the_tag_name_and_stop_at_the_end() {
    assert_eq!(names("<input value=x>"), ["value"]);
    assert_eq!(names("<a  href=x   title='t'>"), ["href", "title"]);
    assert_eq!(names("<br /"), Vec::<&str>::new());
    assert_eq!(names("<input disabled"), ["disabled"]);
    assert_eq!(names("<input"), Vec::<&str>::new());
    assert_eq!(names("<img src = \"a.png\" alt>"), ["src", "alt"]);
    assert_eq!(
        attribute_value("<img src = \"a.png\" alt>", "SRC"),
        Some("a.png")
    );
    assert_eq!(
        attribute_value("<input value=plain>", "value"),
        Some("plain")
    );
    assert_eq!(attribute_value("<input value='open", "value"), Some("open"));
    assert_eq!(attribute_value("<input disabled>", "disabled"), None);
}

#[test]
fn unquoted_values_end_at_whitespace_or_the_closing_bracket() {
    assert_eq!(
        attribute_value("<input value=plain other>", "value"),
        Some("plain")
    );
    assert_eq!(
        attribute_value("<input value=plain>", "value"),
        Some("plain")
    );
    assert_eq!(attribute_value("<input value=>", "value"), Some(""));
    assert_eq!(
        attribute_value("<input id=a value = 'q r'>", "value"),
        Some("q r")
    );
}

#[test]
fn only_non_empty_values_are_masked() {
    assert_eq!(
        mask_attribute(r#"<script nonce="">"#, "nonce", NONCE_PLACEHOLDER),
        r#"<script nonce="">"#
    );
    assert_eq!(
        mask_attribute(
            r#"<script nonce="abc123" src="/a.js">"#,
            "NONCE",
            NONCE_PLACEHOLDER
        ),
        r#"<script nonce="{NONCE}" src="/a.js">"#
    );
    assert_eq!(
        mask_attribute("<script src=/a.js>", "nonce", NONCE_PLACEHOLDER),
        "<script src=/a.js>"
    );
}

#[test]
fn every_closed_nonce_source_is_masked() {
    assert_eq!(
        mask_nonce_sources("script-src 'nonce-abc' 'self' 'nonce-def'"),
        "script-src 'nonce-{NONCE}' 'self' 'nonce-{NONCE}'"
    );
    assert_eq!(mask_nonce_sources("x 'nonce-open"), "x 'nonce-open");
    assert_eq!(mask_nonce_sources("no source"), "no source");
}

#[test]
fn csrf_header_values_are_masked_around_spaced_colons() {
    assert_eq!(
        mask_csrf_headers(r#"<div hx-headers='{"X-CSRF-Token" :  "abc", "x-csrf-token":"def"}'>"#),
        r#"<div hx-headers='{"X-CSRF-Token" :  "{CSRF_TOKEN}", "x-csrf-token":"{CSRF_TOKEN}"}'>"#
    );
    // A key without a colon, or inside a masked value, is left alone.
    assert_eq!(
        mask_csrf_headers(
            r#"<p title="x-csrf-token" data-h='{"X-CSRF-Token": "x-csrf-token: 'v'"}'>"#
        ),
        r#"<p title="x-csrf-token" data-h='{"X-CSRF-Token": "{CSRF_TOKEN}"}'>"#
    );
    assert_eq!(
        mask_csrf_headers(r#"<p data-h='{"X-CSRF-Token": 7}'>"#),
        r#"<p data-h='{"X-CSRF-Token": 7}'>"#
    );
}

#[test]
fn empty_comments_and_text_runs_advance() {
    let plain = SnapshotOptions::new();
    assert_eq!(normalize_html("<!---->x", plain), "<!---->x\n");
    assert_eq!(normalize_html("<p>a  b</p>", plain), "<p>a b</p>\n");
    assert_eq!(
        normalize_html("\u{e9}t\u{e9} <b>x</b>", plain),
        "\u{e9}t\u{e9} <b>x</b>\n"
    );
    assert_eq!(
        normalize_html("<pre> a  b </pre><p>c</p>", plain),
        "<pre> a  b </pre>\n<p>c</p>\n"
    );
}
