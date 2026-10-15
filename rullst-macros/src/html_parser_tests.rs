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

fn attribute_values(source: &str) -> Vec<(String, &'static str)> {
    use super::attributes::HtmlAttrValue;
    let document = parse_ok(source);
    let [HtmlNode::Element(element)] = &document.nodes[..] else {
        panic!("expected one element for {source}");
    };
    element
        .attributes
        .iter()
        .map(|attribute| {
            let kind = match attribute.value {
                HtmlAttrValue::Static(_) => "static",
                HtmlAttrValue::Dynamic(_) => "dynamic",
                HtmlAttrValue::Bare => "bare",
                HtmlAttrValue::Optional(_) => "optional",
            };
            (attribute.name.clone(), kind)
        })
        .collect()
}

#[test]
fn bare_and_conditional_attributes_parse_beside_valued_ones() {
    assert_eq!(
        attribute_values(
            r#"<input required type="checkbox" checked?={done} data-x={1} hx-boost disabled />"#
        ),
        [
            ("required".to_string(), "bare"),
            ("type".to_string(), "static"),
            ("checked".to_string(), "optional"),
            ("data-x".to_string(), "dynamic"),
            ("hx-boost".to_string(), "bare"),
            ("disabled".to_string(), "bare"),
        ]
    );
    assert_eq!(
        attribute_values("<option selected>\"A\"</option>"),
        [("selected".to_string(), "bare")]
    );
    assert_eq!(
        attribute_values("<a aria-current?={page} href?={url}></a>"),
        [
            ("aria-current".to_string(), "optional"),
            ("href".to_string(), "optional"),
        ]
    );
}

#[test]
fn conditional_attributes_need_a_braced_expression() {
    for source in [r#"<input checked?="checked" />"#, "<input checked?=done />"] {
        assert!(
            parse_error(source).starts_with(
                "a conditional attribute takes a `bool` or `Option` expression in braces, \
                 such as `checked?={is_set}`"
            ),
            "{source}"
        );
    }
    // `?` must be followed by `=`.
    assert_eq!(parse_error("<input checked? />"), "expected `=`");
}

#[test]
fn conditional_event_handlers_are_rejected_like_dynamic_ones() {
    for (source, name) in [
        ("<button onclick?={handler}></button>", "onclick"),
        ("<div hx-on-click?={handler}></div>", "hx-on-click"),
    ] {
        assert!(
            parse_error(source).starts_with(&format!("dynamic `{name}` values are not allowed")),
            "{source}"
        );
    }
    // A bare handler name is static (an empty handler) and allowed.
    assert_eq!(
        attribute_values("<button onclick></button>"),
        [("onclick".to_string(), "bare")]
    );
}

#[test]
fn attribute_names_cannot_carry_markup() {
    for source in [
        r#"<input "required" />"#,
        r#"<input x"y"="z" />"#,
        "<input a=b />",
    ] {
        assert!(syn::parse_str::<HtmlDocument>(source).is_err(), "{source}");
    }
}

#[test]
fn control_blocks_parse_with_else_chains_and_patterns() {
    let document = parse_ok(
        r#"@if let Some(user) = user { <b>{user}</b> } @else if admin { "admin" } @else { "guest" }
           @for (index, item) in items.iter().enumerate() { <li data-i={index}>{item}</li> }
           @if flag { <hr /> }"#,
    );
    let [
        HtmlNode::If(chain),
        HtmlNode::For(for_node),
        HtmlNode::If(single),
    ] = &document.nodes[..]
    else {
        panic!("expected @if, @for and @if nodes");
    };
    assert_eq!(chain.branches.len(), 2);
    assert!(matches!(chain.branches[0].0, syn::Expr::Let(_)));
    assert_eq!(chain.otherwise.as_ref().map(Vec::len), Some(1));
    assert!(matches!(for_node.pattern, syn::Pat::Tuple(_)));
    assert_eq!(for_node.body.len(), 1);
    assert_eq!(single.branches.len(), 1);
    assert!(single.otherwise.is_none());

    // Control blocks nest inside elements and each other.
    let document =
        parse_ok("<ul>@for row in rows { @if row.visible { <li>{row.name}</li> } }</ul>");
    let [HtmlNode::Element(list)] = &document.nodes[..] else {
        panic!("expected one <ul>");
    };
    let [HtmlNode::For(rows)] = &list.children[..] else {
        panic!("expected one @for child");
    };
    assert!(matches!(rows.body[..], [HtmlNode::If(_)]));
}

#[test]
fn malformed_control_blocks_are_rejected() {
    assert_eq!(
        parse_error(r#"@if a { "x" } else { "y" }"#),
        "write `@else` after an `@if` block in html!"
    );
    assert_eq!(
        parse_error(r#"<p>@else { "y" }</p>"#),
        "`@else` must directly follow the body of an `@if` or `@else if` block"
    );
    for source in ["@match x { _ => {} }", "@while x { }", "@let x = 1;"] {
        assert!(
            parse_error(source).starts_with("html! supports `@if`, `@else` and `@for`"),
            "{source}"
        );
    }
    assert_eq!(
        parse_error("<div>@if a { </div> }</div>"),
        "unexpected closing tag in an `@if`/`@for` body"
    );
    assert_eq!(
        parse_error("@if a { <!DOCTYPE html> }"),
        "`<!DOCTYPE html>` must come before any other content in html!"
    );
    assert!(syn::parse_str::<HtmlDocument>("@for x items { }").is_err());
    assert!(syn::parse_str::<HtmlDocument>(r#"@if a "x""#).is_err());
}
