use super::{HtmlDocument, HtmlNode};

fn parse_error(source: &str) -> String {
    match syn::parse_str::<HtmlDocument>(source) {
        Ok(_) => panic!("expected a parse error for {source}"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn dynamic_event_handler_attributes_are_rejected() {
    for (source, name) in [
        (r#"<button onclick={handler}>"Go"</button>"#, "onclick"),
        (r#"<body ONLOAD={handler}></body>"#, "ONLOAD"),
        (r#"<div hx-on={handler}></div>"#, "hx-on"),
        (r#"<div hx-on-click={handler}></div>"#, "hx-on-click"),
        (
            r#"<div data-hx-on-click={handler}></div>"#,
            "data-hx-on-click",
        ),
    ] {
        let error = parse_error(source);
        assert!(
            error.starts_with(&format!("dynamic `{name}` values are not allowed")),
            "{error}"
        );
    }
}

#[test]
fn static_handlers_and_other_dynamic_attributes_are_accepted() {
    for source in [
        r#"<button onclick="go()">"Go"</button>"#,
        r#"<div hx-on-click="go()"></div>"#,
        r#"<div data-on={value} one-time={value} on={value}></div>"#,
        r#"<a href={url} style={style}>"x"</a>"#,
    ] {
        assert!(syn::parse_str::<HtmlDocument>(source).is_ok(), "{source}");
    }
}

fn parse_ok(source: &str) -> HtmlDocument {
    match syn::parse_str::<HtmlDocument>(source) {
        Ok(document) => document,
        Err(error) => panic!("expected {source} to parse: {error}"),
    }
}

#[test]
fn rust_keywords_are_accepted_as_element_names() {
    let document = parse_ok(r##"<svg><use href="#i"/></svg>"##);
    let [HtmlNode::Element(svg)] = &document.nodes[..] else {
        panic!("expected one <svg> element");
    };
    assert_eq!(svg.tag_name.to_string(), "svg");
    let [HtmlNode::Element(use_element)] = &svg.children[..] else {
        panic!("expected one <use> child");
    };
    assert_eq!(use_element.tag_name.to_string(), "use");
    assert!(syn::parse_str::<HtmlDocument>("<use></use>").is_ok());
    assert_eq!(
        parse_error("<use></type>"),
        "Mismatched closing tag: expected </use>, found </type>"
    );
}

#[test]
fn the_html5_doctype_becomes_a_verbatim_text_node() {
    for (source, declaration) in [
        ("<!DOCTYPE html><html></html>", "<!DOCTYPE html>"),
        ("<!doctype HTML><html></html>", "<!doctype HTML>"),
        // Comments are dropped, so one may precede the declaration.
        (
            "<!-- generated --><!DOCTYPE html><html></html>",
            "<!DOCTYPE html>",
        ),
    ] {
        let document = parse_ok(source);
        let [HtmlNode::Text(text), HtmlNode::Element(html)] = &document.nodes[..] else {
            panic!("expected the declaration and <html> for {source}");
        };
        assert_eq!(text.value(), declaration);
        assert_eq!(html.tag_name.to_string(), "html");
    }
}

#[test]
fn other_declarations_are_rejected() {
    for source in [
        "<!DOCTYPO html><html></html>",
        "<!DOCTYPE svg><html></html>",
        r#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN"><html></html>"#,
    ] {
        assert_eq!(
            parse_error(source),
            "html! accepts only the HTML5 declaration `<!DOCTYPE html>`",
            "{source}"
        );
    }
}

#[test]
fn the_doctype_must_come_first() {
    for source in [
        "<p></p><!DOCTYPE html>",
        r#""text"<!DOCTYPE html>"#,
        "<div><!DOCTYPE html></div>",
    ] {
        assert_eq!(
            parse_error(source),
            "`<!DOCTYPE html>` must come before any other content in html!",
            "{source}"
        );
    }
}
