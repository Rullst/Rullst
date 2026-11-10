use super::*;

const THEME: &str = "#0f172a";

fn wired(source: &str) -> Wiring {
    wire(source, THEME, true).unwrap()
}

#[test]
fn html_documents_get_the_head_call_once() {
    let source = r#"
// A comment mentioning </head> is not a layout.
/// Neither is documentation: <head></head>
pub fn page(nonce: &str) -> Markup {
    rullst::html! {
        <html><head><title>"T"</title><style nonce={nonce}>"p{}"</style></head><body><header>"h"</header></body></html>
    }
}
"#;
    let first = wired(source);
    assert_eq!((first.wired, first.present), (1, 0));
    assert!(
        first
            .source
            .contains(r#"<style nonce={nonce}>"p{}"</style>{crate::pwa::head()}</head>"#),
        "{}",
        first.source
    );
    assert!(first.source.contains("// A comment mentioning </head> is not a layout.\n/// Neither is documentation: <head></head>\n"));
    let second = wired(&first.source);
    assert_eq!((second.wired, second.present), (0, 1));
    assert_eq!(second.source, first.source);
}

#[test]
fn only_html_macro_bodies_and_own_manifest_links_count() {
    // `<` `/` `head` `>` outside html! is not markup.
    let comparison = "fn f(a: u8, head: u8) -> bool { a < head && head > 2 }";
    assert_eq!(wired(comparison).wired, 0);
    // An own manifest without a worker registration gets only the script.
    let own =
        r#"fn f() -> Markup { html! { <head><link rel="manifest" href="/m.json" /></head> } }"#;
    let result = wired(own);
    assert_eq!(
        (result.wired, result.own_manifest, result.present),
        (1, 1, 0)
    );
    assert_eq!(
        result.source,
        r#"fn f() -> Markup { html! { <head><link rel="manifest" href="/m.json" /><script src="/static/pwa.js" defer></script></head> } }"#
    );
    let again = wired(&result.source);
    assert_eq!((again.wired, again.present), (0, 1));
    // A head that registers the worker itself is complete.
    for registered in [
        r#"fn f() -> Markup { html! { <head><script src="/static/pwa.js" defer></script></head> } }"#,
        r#"fn f() -> Markup { html! { <head><script src="/static/register-sw.js"></script>{crate::pwa::head()}</head> } }"#,
    ] {
        let result = wired(registered);
        assert_eq!((result.wired, result.present), (0, 1), "{registered}");
        assert_eq!(result.source, registered);
    }
    let other_rel =
        r#"fn f() -> Markup { html! { <head><link rel="icon" href="/i.png" /></head> } }"#;
    assert_eq!(wired(other_rel).wired, 1);
    // Two documents in one file, one inside a nested block.
    let two =
        "fn f() -> Markup { html! { <head></head> @if true { <html><head></head></html> } } }";
    let result = wired(two);
    assert_eq!(result.wired, 2);
    assert_eq!(
        result.source.matches("{crate::pwa::head()}</head>").count(),
        2
    );
}

#[test]
fn string_documents_get_single_quoted_tags_without_breaking_the_literal() {
    let tags = literal_tags(THEME);
    assert_eq!(
        tags,
        "<link rel='manifest' href='/static/manifest.webmanifest'><meta name='theme-color' content='#0f172a'><link rel='apple-touch-icon' href='/static/icons/apple-touch-icon.png'><script src='/static/pwa.js' defer></script>"
    );
    for forbidden in ['"', '{', '}', '\\'] {
        assert!(!tags.contains(forbidden));
    }
    let source = concat!(
        "fn a(nonce: &str) -> String { format!(r#\"<!doctype html><html><HEAD><style nonce=\"{nonce}\">b{{}}</style></HEAD></html>\"#) }\n",
        "fn b() -> &'static str { \"<html><head><title>x</title></head></html>\" }\n",
        "fn c() -> &'static [u8] { b\"<head></head>\" }\n",
    );
    let result = wired(source);
    assert_eq!((result.wired, result.present), (2, 0));
    assert!(
        result
            .source
            .contains(&format!("</style>{tags}</HEAD></html>\"#)")),
        "{}",
        result.source
    );
    assert!(
        result
            .source
            .contains(&format!("<title>x</title>{tags}</head>"))
    );
    assert!(result.source.contains("b\"<head></head>\""));
    let again = wired(&result.source);
    assert_eq!((again.wired, again.present), (0, 2));
    assert_eq!(again.source, result.source);
}

#[test]
fn core_layout_calls_gain_the_head_argument_and_their_import() {
    let source = r#"use rullst::htmx::{HtmxRequest, render_page_with_lang};

async fn home(htmx: HtmxRequest) -> impl IntoResponse {
    render_page_with_lang(&htmx, "en", "Welcome", content)
}

async fn other(htmx: HtmxRequest) -> impl IntoResponse {
    rullst::htmx::render_page_with_lang(&htmx,"en","Other",html! { <p>"x"</p> },)
}
"#;
    let result = wired(source);
    assert_eq!((result.wired, result.present), (2, 0));
    assert!(
        result
            .source
            .starts_with("use rullst::htmx::{HtmxRequest, render_page_with_head};")
    );
    assert!(
        result.source.contains(
            r#"render_page_with_head(&htmx, "en", "Welcome", crate::pwa::head(), content)"#
        )
    );
    // Past rustfmt's 60-column argument width, one argument per line.
    assert!(
        result.source.contains(
            "    rullst::htmx::render_page_with_head(\n        &htmx,\n        \"en\",\n        \"Other\",\n        crate::pwa::head(),\n        html! { <p>\"x\"</p> },\n    )\n}"
        ),
        "{}",
        result.source
    );
    assert!(!result.source.contains("render_page_with_lang"));
    let again = wired(&result.source);
    assert_eq!((again.wired, again.present), (0, 2));
    assert_eq!(again.source, result.source);
    // The application's own head markup is not the PWA's.
    let own = "fn f() { render_page_with_head(&htmx, \"en\", \"T\", mine(), content); }";
    assert_eq!((wired(own).wired, wired(own).present), (0, 0));
}

#[test]
fn other_uses_of_the_layout_name_are_left_alone() {
    let other_uses = [
        // A local function of the same name, its definition and a method.
        "fn render_page_with_lang(a: u8, b: u8, c: u8, d: u8) {}",
        "fn f(x: X) { x.render_page_with_lang(1, 2, 3, 4); }",
        // Not four arguments.
        "fn f() { render_page_with_lang(1, 2, 3); }",
        "fn f() { render_page_with_lang(1, 2, 3, 4, 5); }",
        "fn f() { let g = render_page_with_lang; }",
    ];
    for source in other_uses {
        let result = wired(source);
        assert_eq!(result.wired, 0, "{source}");
        assert_eq!(result.source, source);
    }
    // The import stays while another use still needs it, or when the new
    // name is already imported.
    let still_used = "use rullst::render_page_with_lang;\nfn f() { render_page_with_lang(1, 2, 3, 4); let g = render_page_with_lang; }";
    let result = wired(still_used);
    assert_eq!(result.wired, 1);
    assert!(
        result
            .source
            .starts_with("use rullst::render_page_with_lang;")
    );
    let both = "use rullst::{render_page_with_head, render_page_with_lang};\nfn f() { render_page_with_lang(1, 2, 3, 4); }";
    let result = wired(both);
    assert!(
        result
            .source
            .starts_with("use rullst::{render_page_with_head, render_page_with_lang};")
    );
    // A call already written one argument per line keeps that layout.
    let vertical = "fn f() {\n    render_page_with_lang(\n        &htmx,\n        \"en\",\n        \"T\",\n        content,\n    );\n}";
    assert_eq!(
        wired(vertical).source,
        "fn f() {\n    render_page_with_head(\n        &htmx,\n        \"en\",\n        \"T\",\n        crate::pwa::head(),\n        content,\n    );\n}"
    );
    // Within a longer expression only the argument is added.
    assert_eq!(
        wired("fn f() { let page = render_page_with_lang(a, b, c, d); }").source,
        "fn f() { let page = render_page_with_head(a, b, c, crate::pwa::head(), d); }"
    );
    // A line that would pass 100 columns is split even with short arguments.
    let deep = format!(
        "fn f() {{\n{}render_page_with_lang(a, b, c, d)\n}}",
        " ".repeat(70)
    );
    assert!(wired(&deep).source.contains("render_page_with_head(\n"));
    // rustfmt keeps a call on one line up to 60 columns of arguments and a
    // 100-column line.
    let title = |length: usize| format!("\"{}\"", "t".repeat(length - 2));
    let call = |indent: usize, title: &str| {
        format!(
            "fn f() {{\n{}render_page_with_lang(&htmx, \"en\", {title}, content)\n}}",
            " ".repeat(indent)
        )
    };
    // `&htmx, "en", <title>, crate::pwa::head(), content` is 42 columns plus the title.
    assert!(wired(&call(4, &title(18))).source.contains(&format!(
        "    render_page_with_head(&htmx, \"en\", {}, crate::pwa::head(), content)\n",
        title(18)
    )));
    assert!(
        wired(&call(4, &title(19)))
            .source
            .contains("render_page_with_head(\n")
    );
    // With 30 columns of arguments, the line is 53 columns plus the indent.
    let short = |indent: usize| {
        wired(&format!(
            "fn f() {{\n{}render_page_with_lang(a, b, c, d)\n}}",
            " ".repeat(indent)
        ))
        .source
    };
    assert!(short(47).contains("render_page_with_head(a, b, c, crate::pwa::head(), d)\n"));
    assert!(short(48).contains("render_page_with_head(\n"));
    // A path after other code is not a call that starts its line.
    let assigned = "fn f() { let page = rullst::htmx::render_page_with_lang(&htmx, \"en\", \"A title long enough to wrap\", content); }";
    assert_eq!(
        wired(assigned).source,
        "fn f() { let page = rullst::htmx::render_page_with_head(&htmx, \"en\", \"A title long enough to wrap\", crate::pwa::head(), content); }"
    );
    // A trailing comma after four arguments is still the layout call.
    assert_eq!(
        wired("fn f() { render_page_with_lang(1, 2, 3, 4,); }").wired,
        1
    );
}

#[test]
fn string_heads_count_only_a_worker_registration_as_done() {
    // A hand-written head with its own manifest gains only the script.
    let own = "fn page() -> String { \"<html><head><link rel='manifest' href='/site.webmanifest'></head></html>\".to_owned() }";
    let result = wired(own);
    assert_eq!(
        (result.wired, result.own_manifest, result.present),
        (1, 1, 0)
    );
    assert!(result.source.contains(
        "<link rel='manifest' href='/site.webmanifest'><script src='/static/pwa.js' defer></script></head>"
    ));
    assert_eq!(
        (wired(&result.source).wired, wired(&result.source).present),
        (0, 1)
    );
    let registered = "fn page() -> &'static str { \"<head><script>navigator.serviceWorker.register('/sw.js')</script></head>\" }";
    assert_eq!((wired(registered).wired, wired(registered).present), (0, 1));
}

#[test]
fn search_patterns_and_test_code_are_not_layouts() {
    let source = r#"
fn inject(body: &str) -> String {
    body.replacen("</head>", "<script src='/live.js'></script></head>", 1)
}
fn header() -> &'static str { "<header></header></head>" }

#[test]
fn layout_has_a_head() {
    assert!(page().contains("<head><title>x</title></head>"));
}

#[cfg(test)]
mod tests {
    fn fixture() -> Markup { html! { <html><head></head></html> } }
    const PAGE: &str = "<html><head></head></html>";
}

#[cfg(test)]
const FIXTURE: &str = "<head></head>";

fn page() -> &'static str { "<html><head><title>x</title></head></html>" }
"#;
    let result = wired(source);
    assert_eq!((result.wired, result.present), (1, 0), "{}", result.source);
    assert!(result.source.ends_with(&format!(
        "\"<html><head><title>x</title>{}</head></html>\" }}\n",
        literal_tags(THEME)
    )));
    assert_eq!(result.source.matches("pwa.js").count(), 1);
    // A whole file compiled only for tests is left alone.
    let test_file = "#![cfg(test)]\nfn page() -> &'static str { \"<head></head>\" }\n";
    assert_eq!(wired(test_file).wired, 0);
}

#[test]
fn string_documents_of_a_file_that_sends_mail_are_left_alone() {
    let source = "fn message() -> String { \"<html><head><title>Receipt</title></head></html>\".to_owned() }\nfn page() -> Markup { html! { <html><head></head></html> } }\n";
    let result = wire(source, THEME, false).unwrap();
    assert_eq!((result.wired, result.skipped), (1, 1));
    assert!(result.source.contains("<title>Receipt</title></head>"));
    assert!(result.source.contains("{crate::pwa::head()}</head>"));
    assert_eq!(wired(source).skipped, 0);
}

#[test]
fn unparsable_sources_are_refused() {
    assert!(wire("fn f( {", THEME, true).is_err());
}
