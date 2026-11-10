//! Unquoted text and the Alpine.js/htmx attribute names (13.0.0-alpha.5).
//! `syn::parse_str` keeps source positions (`span-locations`), as a real
//! expansion does.

use super::{HtmlDocument, HtmlNode};

fn parse(source: &str) -> HtmlDocument {
    match syn::parse_str::<HtmlDocument>(source) {
        Ok(document) => document,
        Err(error) => panic!("expected {source} to parse: {error}"),
    }
}

fn error(source: &str) -> String {
    match syn::parse_str::<HtmlDocument>(source) {
        Ok(_) => panic!("expected a parse error for {source}"),
        Err(error) => error.to_string(),
    }
}

/// The text nodes of the first element, as the expansion writes them.
fn texts(source: &str) -> Vec<String> {
    let document = parse(source);
    let Some(HtmlNode::Element(element)) = document.nodes.first() else {
        panic!("expected an element in {source}");
    };
    element
        .children
        .iter()
        .map(|node| match node {
            HtmlNode::Text(text) => text.value(),
            HtmlNode::Block(_) => "{…}".to_string(),
            HtmlNode::Element(element) => format!("<{}>", element.tag_name),
            _ => "@".to_string(),
        })
        .collect()
}

#[test]
fn words_and_punctuation_keep_the_spacing_of_the_source() {
    assert_eq!(texts("<p>Hello, world!</p>"), ["Hello, world!"]);
    assert_eq!(
        texts("<p>\n    Many   spaces\n    and lines  </p>"),
        ["Many spaces and lines"]
    );
    assert_eq!(
        texts("<p>12:30, 1.5% & 3rd; a-b/c?</p>"),
        ["12:30, 1.5% & 3rd; a-b/c?"]
    );
    assert_eq!(
        texts("<p>Hi {name}, you have {count} new</p>"),
        ["Hi ", "{…}", ", you have ", "{…}", " new"]
    );
    assert_eq!(texts("<p>{a}{b} {c}</p>"), ["{…}", "{…}", "{…}"]);
    assert_eq!(texts("<p>({count})</p>"), ["(", "{…}", ")"]);
    assert_eq!(texts("<p>[ draft ] (a, b)</p>"), ["[draft] (a, b)"]);
    assert_eq!(texts("<p>Read <a>more</a>.</p>"), ["Read ", "<a>", "."]);
    assert_eq!(
        texts("<p>\n  Read\n  <a>more</a>\n  now\n</p>"),
        ["Read ", "<a>", " now"]
    );
    assert_eq!(texts(r#"<p>Say "hi" now</p>"#), ["Say hi now"]);
    assert_eq!(texts(r#"<p>"a" {b} "c"</p>"#), ["a", "{…}", "c"]);
    assert_eq!(texts("<p>{name}'s</p>"), ["{…}", "'s"]);
    assert_eq!(texts("<p>Tom &amp; Jerry</p>"), ["Tom &amp; Jerry"]);
    assert_eq!(texts("<p>if else for match</p>"), ["if else for match"]);
    assert_eq!(
        texts("<p>Status: @if on { done } today</p>"),
        ["Status: ", "@", " today"]
    );
    // A comment between words counts as whitespace.
    assert_eq!(texts("<p>one /* two */ three</p>"), ["one three"]);
    assert_eq!(texts("<p>one<!-- two -->three</p>"), ["onethree"]);
}

#[test]
fn whitespace_at_the_edges_is_dropped_everywhere() {
    let document = parse("  Hello  <b> bold </b>  ");
    let [HtmlNode::Text(text), HtmlNode::Element(bold)] = &document.nodes[..] else {
        panic!("expected text and <b>");
    };
    assert_eq!(text.value(), "Hello ");
    let [HtmlNode::Text(inner)] = &bold.children[..] else {
        panic!("expected one text child");
    };
    assert_eq!(inner.value(), "bold");
    let document = parse("@if a { Yes } @else {  No  }");
    let [HtmlNode::If(chain)] = &document.nodes[..] else {
        panic!("expected @if");
    };
    let [HtmlNode::Text(yes)] = &chain.branches[0].1[..] else {
        panic!("expected text in @if");
    };
    assert_eq!(yes.value(), "Yes");
    let otherwise = chain.otherwise.as_deref().unwrap_or_default();
    let [HtmlNode::Text(no)] = otherwise else {
        panic!("expected text in @else");
    };
    assert_eq!(no.value(), "No");
}

#[test]
fn generated_tokens_without_positions_are_separated_by_spaces() {
    let tokens = quote::quote! { <p>Hello, world!</p> };
    let document = syn::parse2::<HtmlDocument>(tokens).unwrap_or_else(|e| panic!("{e}"));
    let [HtmlNode::Element(paragraph)] = &document.nodes[..] else {
        panic!("expected <p>");
    };
    let [HtmlNode::Text(text)] = &paragraph.children[..] else {
        panic!("expected one text child");
    };
    assert_eq!(text.value(), "Hello , world !");
}

#[test]
fn text_that_rust_or_html_syntax_owns_is_rejected_with_its_fix() {
    for (source, message) in [
        ("<p>a > b</p>", "`>` is tag syntax in html!"),
        ("<p>x 'y' z</p>", "character and byte literals are not text"),
        (
            r#"<p>bytes b"x"</p>"#,
            "character and byte literals are not text",
        ),
        (
            "<p>Visit https://example.com today</p>",
            "`//` starts a Rust comment",
        ),
        ("<p>\n  see http://x\n</p>", "`//` starts a Rust comment"),
        (
            "<p>/// a doc\nText</p>",
            "doc comments (`///`, `//!`) are not text",
        ),
        (
            "<div>if ok { <p>yes</p> }</div>",
            "write `@if` for a control block in html!",
        ),
        (
            "<ul>for row in rows { <li>{row}</li> }</ul>",
            "write `@for` for a control block in html!",
        ),
        ("<p>( </p> )</p>", "unexpected closing tag inside `(…)`"),
    ] {
        let found = error(source);
        assert!(found.starts_with(message), "{source}: {found}");
    }
    // An e-mail address stops at `@`, which starts a control block.
    assert!(
        error("<p>ana@example.com</p>").contains("write a literal `@`"),
        "{}",
        error("<p>ana@example.com</p>")
    );
    // `else` after an `@if` body is still the `@else` mistake.
    assert_eq!(
        error(r#"@if a { "x" } else if b { "y" }"#),
        "write `@else` after an `@if` block in html!"
    );
    // Text that only looks like a scheme, or keeps going, is accepted.
    assert_eq!(texts("<p>Note: http is plain</p>"), ["Note: http is plain"]);
    assert_eq!(texts("<p>if {ok}</p>"), ["if ", "{…}"]);
}

/// The attribute names of the first element.
fn names(source: &str) -> Vec<String> {
    let document = parse(source);
    let Some(HtmlNode::Element(element)) = document.nodes.first() else {
        panic!("expected an element in {source}");
    };
    element
        .attributes
        .iter()
        .map(|attribute| attribute.name.clone())
        .collect()
}

#[test]
fn alpine_and_htmx_names_parse_with_colons_at_signs_and_dots() {
    assert_eq!(
        names(
            r##"<div x-data="{}" @click.outside="o" :class="c" x-on:keyup.enter.prevent="s"
                    x-bind:disabled="b" hx-on::after-request="r" hx-on:htmx:before-swap="q"
                    x-transition:enter.duration.500ms xlink:href="#i" data-2x="d"></div>"##
        ),
        [
            "x-data",
            "@click.outside",
            ":class",
            "x-on:keyup.enter.prevent",
            "x-bind:disabled",
            "hx-on::after-request",
            "hx-on:htmx:before-swap",
            "x-transition:enter.duration.500ms",
            "xlink:href",
            "data-2x",
        ]
    );
    // A `:` written after a space starts the next attribute.
    assert_eq!(
        names(r#"<input required :value="v" disabled:x="y" />"#),
        ["required", ":value", "disabled:x"]
    );
    assert_eq!(
        names(r#"<a @click="go" type="button"></a>"#),
        ["@click", "type"]
    );
    // Keywords are segments too.
    assert_eq!(
        names(r#"<p :type="t" @click.self="s"></p>"#),
        [":type", "@click.self"]
    );
}

#[test]
fn malformed_names_and_dynamic_script_values_are_rejected() {
    for source in [
        r#"<div x-on:="x"></div>"#,
        r#"<div @="x"></div>"#,
        r#"<div x.="x"></div>"#,
        r#"<div ::x="x"></div>"#,
        r#"<div x.1.5s="x"></div>"#,
    ] {
        assert!(syn::parse_str::<HtmlDocument>(source).is_err(), "{source}");
    }
    assert!(
        error(r#"<div x-on:="x"></div>"#)
            .starts_with("expected the rest of the attribute name after `:`")
    );
    for (source, name) in [
        ("<div x-data={state}></div>", "x-data"),
        ("<div x-show?={open}></div>", "x-show"),
        ("<div @click={handler}></div>", "@click"),
        ("<div :class={classes}></div>", ":class"),
        ("<div x-bind:href={url}></div>", "x-bind:href"),
        ("<div hx-on:click={handler}></div>", "hx-on:click"),
        (
            "<div hx-on::after-request={handler}></div>",
            "hx-on::after-request",
        ),
        ("<div data-hx-on:click={handler}></div>", "data-hx-on:click"),
        ("<div on:click={handler}></div>", "on:click"),
    ] {
        let found = error(source);
        assert!(
            found.starts_with(&format!("dynamic `{name}` values are not allowed")),
            "{source}: {found}"
        );
    }
    // Static values stay author-owned, and other names keep dynamic values.
    for source in [
        r#"<div x-data="{ open: false }" @click="open = !open" :class="{ a: open }"></div>"#,
        "<div aria-label:x={label} xlink:href={url} data-x={value}></div>",
        "<div x-cloak hx-on></div>",
    ] {
        assert!(syn::parse_str::<HtmlDocument>(source).is_ok(), "{source}");
    }
}
