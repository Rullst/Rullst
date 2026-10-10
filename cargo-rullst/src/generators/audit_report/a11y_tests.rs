#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::generators::audit_compliance::EvidenceStatus;
use crate::generators::audit_report::tests::project;

fn run(files: &[(&str, &str)]) -> Vec<Check> {
    let root = project(files);
    let sources = ProjectSources::load(root.path());
    let (markup, incomplete) = collect(root.path(), &sources);
    checks(&markup, &incomplete)
}

fn locations(check: &Check) -> Vec<String> {
    check.findings.iter().map(Finding::location).collect()
}

#[test]
fn html_macro_bodies_are_extracted_with_nested_braces_and_strings() {
    let source = "fn a() -> Html {\n    html! { <p class={x}>{ \"}\" }</p> }\n}\nfn b() { my_html!(x); html!(<br />) }\n";
    let bodies = html_macro_bodies(source);
    assert_eq!(bodies.len(), 2);
    assert_eq!(bodies[0].1.trim(), "<p class={x}>{ \"}\" }</p>");
    assert_eq!(bodies[1].1, "<br />");
}

#[test]
fn tags_parse_quoted_braced_and_bare_attributes() {
    let tags = tags(
        "<!-- <img> --><!DOCTYPE html><input type=\"text\" id={field_id} required disabled=\"true\" data-x=1></label>",
    );
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].name, "input");
    assert_eq!(tags[0].attribute("id"), Some(Some("{field_id}")));
    assert_eq!(tags[0].attribute("required"), Some(None));
    assert_eq!(tags[0].attribute("data-x"), Some(Some("1")));
    assert!(tags[1].closing && tags[1].name == "label");
}

#[test]
fn images_without_alt_are_reported_with_their_line() {
    let checks = run(&[(
        "src/views.rs",
        "fn page() -> Html {\n    html! {\n        <img src=\"/a.png\" alt=\"Logo\" />\n        <img src={url} />\n    }\n}\n",
    )]);
    assert_eq!(checks[0].spec.id, "a11y_img_alt");
    assert_eq!(locations(&checks[0]), ["src/views.rs:4"]);
    assert!(checks[0].detail.starts_with("2 <img> tag(s)"));
}

#[test]
fn form_controls_accept_for_wrapping_and_aria_labels() {
    let labelled = r#"<form>
<label for="email">Email</label><input id="email" type="email">
<label>Name <input name="name"></label>
<select aria-label="Plan"></select>
<textarea aria-labelledby="notes-title"></textarea>
<input type="hidden" name="_token"><input type="submit" value="Send">
<label for={field}>Dynamic</label><input id={field}>
</form>"#;
    let checks = run(&[("templates/form.html", labelled)]);
    assert_eq!(
        checks[1].status,
        EvidenceStatus::NoFindings,
        "{:?}",
        checks[1].findings
    );

    let unlabelled = "<label for=\"other\">Other</label>\n<input id=\"email\" type=\"email\">\n<select name=\"plan\"></select>\n<textarea></textarea>\n";
    let checks = run(&[("templates/form.html.tera", unlabelled)]);
    assert_eq!(
        locations(&checks[1]),
        [
            "templates/form.html.tera:2",
            "templates/form.html.tera:3",
            "templates/form.html.tera:4"
        ]
    );
    assert!(checks[1].findings[0].message.contains("`<input>`"));
}

#[test]
fn html_elements_need_a_lang_attribute() {
    let checks = run(&[
        (
            "templates/base.html",
            "<!DOCTYPE html>\n<html>\n<body></body></html>\n",
        ),
        ("src/layout.html", "<html lang=\"pt-BR\"></html>\n"),
    ]);
    assert_eq!(checks[2].spec.id, "a11y_html_lang");
    assert_eq!(locations(&checks[2]), ["templates/base.html:2"]);
}

#[test]
fn projects_without_markup_are_not_checked_and_test_markup_is_ignored() {
    let checks = run(&[(
        "src/main.rs",
        "fn main() {}\n#[cfg(test)]\nmod tests { fn t() { html! { <img src=\"x\"> }; } }\n",
    )]);
    assert_eq!(checks.len(), 3);
    assert!(
        checks
            .iter()
            .all(|check| matches!(check.status, EvidenceStatus::NotChecked(_)))
    );
}

#[test]
fn bodies_carry_their_byte_offset_and_skip_escaped_quotes() {
    let source = "let x = html! { <p>hi</p> };";
    assert_eq!(html_macro_bodies(source), [(15, " <p>hi</p> ")]);

    // An escaped quote does not end the string, so its `}` stays inside.
    let escaped = r#"html! { "a \" }" <b></b> } html!(<i></i>)"#;
    assert_eq!(
        html_macro_bodies(escaped),
        [(7, r#" "a \" }" <b></b> "#), (33, "<i></i>")]
    );
    // A trailing escape inside an unterminated string never closes.
    assert!(html_macro_bodies(r#"html! { "\"#).is_empty());
}

fn attribute_list(tag: &Tag) -> Vec<(&str, Option<&str>)> {
    tag.attributes
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_deref()))
        .collect()
}

#[test]
fn attribute_values_are_parsed_whole() {
    let parsed = tags(
        r#"<img {if x > 1 { "a" } else { "b" }} alt="a b" title= 'c' id={x y} data-v=bare><br>"#,
    );
    assert_eq!(
        parsed.len(),
        2,
        "{:?}",
        parsed.iter().map(|tag| &tag.name).collect::<Vec<_>>()
    );
    assert_eq!(
        attribute_list(&parsed[0]),
        [
            ("alt", Some("a b")),
            ("title", Some("c")),
            ("id", Some("{x y}")),
            ("data-v", Some("bare"))
        ]
    );
    assert_eq!(parsed[1].name, "br");

    // An unterminated tag still yields its attributes.
    let open = tags(r#"<img alt="x" hidden"#);
    assert_eq!(
        attribute_list(&open[0]),
        [("alt", Some("x")), ("hidden", None)]
    );
    let open = tags("<img alt");
    assert_eq!(attribute_list(&open[0]), [("alt", None)]);
}

#[test]
fn only_label_elements_associate_controls_and_counts_are_reported() {
    let page = r#"<html lang="en"><output for="email"></output><input id="email"><input id={field}><img src="a.png" alt=""></html>"#;
    let checks = run(&[("templates/page.html", page)]);
    assert_eq!(locations(&checks[1]).len(), 2, "{:?}", checks[1].findings);
    assert!(
        checks[0].detail.starts_with("1 <img> tag(s);"),
        "{}",
        checks[0].detail
    );
    assert!(
        checks[1]
            .detail
            .starts_with("2 form control(s) needing a label;"),
        "{}",
        checks[1].detail
    );
    assert!(
        checks[2].detail.starts_with("1 <html> element(s);"),
        "{}",
        checks[2].detail
    );
}

#[test]
fn comments_and_names_that_are_not_tags_are_skipped() {
    let names = |text| {
        tags(text)
            .into_iter()
            .map(|tag| tag.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(names("<!--<b>--><i>"), ["i"]);
    assert_eq!(names("a <1x> b < c <p>"), ["p"]);
    assert_eq!(names("<!-- open <b>"), Vec::<String>::new());
    assert_eq!(names("<a href=x><b>"), ["a", "b"]);
    assert_eq!(tags("x <img alt><br>")[1].offset, 11);
}
