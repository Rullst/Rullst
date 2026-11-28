//! The statements the expansion writes, asserted on the generated tokens:
//! text joined into one `push_str` per run, one buffer for the whole
//! document, and a buffer of its own for an `@for` body node that mentions
//! `continue` or `break`.

use proc_macro2::Span;
use syn::Ident;

use super::super::HtmlDocument;
use super::{Writer, expr_mentions_loop_exit};

fn expansion(source: &str) -> String {
    match syn::parse_str::<HtmlDocument>(source) {
        Ok(document) => document.to_tokens().to_string(),
        Err(error) => panic!("expected {source} to parse: {error}"),
    }
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn adjacent_text_is_written_by_one_push() {
    let tokens = expansion(r#"<!DOCTYPE html><html><head><title>"T"</title></head></html>"#);
    assert!(
        tokens.contains(
            r#"s . push_str ("<!DOCTYPE html><html><head><title>T</title></head></html>") ;"#
        ),
        "{tokens}"
    );
    assert_eq!(count(&tokens, "push_str"), 1, "{tokens}");
    assert_eq!(count(&tokens, "String :: with_capacity"), 1, "{tokens}");
}

#[test]
fn values_split_the_text_around_them() {
    let tokens = expansion("<tr><td>{id}</td><td>{message}</td></tr>");
    for expected in [
        r#"s . push_str ("<tr><td>") ;"#,
        "s . push_str (& rullst :: html :: escape (& (id))) ;",
        r#"s . push_str ("</td><td>") ;"#,
        "s . push_str (& rullst :: html :: escape (& (message))) ;",
        r#"s . push_str ("</td></tr>") ;"#,
    ] {
        assert!(tokens.contains(expected), "{expected} in {tokens}");
    }
    assert_eq!(count(&tokens, "String :: with_capacity"), 1, "{tokens}");
    assert!(tokens.ends_with("s })"), "{tokens}");
}

#[test]
fn control_blocks_write_into_the_document_buffer() {
    let tokens = expansion(
        r#"<ul>@for row in rows { <li>{row}</li> }</ul>@if a { "x" } @else { "y" }@match m { 0 => { "z" } _ => {} }"#,
    );
    assert_eq!(count(&tokens, "String :: with_capacity"), 1, "{tokens}");
    assert_eq!(count(&tokens, "String :: new"), 0, "{tokens}");
    for expected in [
        r#"for row in rows { s . push_str ("<li>") ;"#,
        r#"if a { s . push_str ("x") ; } else { s . push_str ("y") ; }"#,
        r#"match m { 0 => { s . push_str ("z") ; } _ => { } }"#,
    ] {
        assert!(tokens.contains(expected), "{expected} in {tokens}");
    }
}

#[test]
fn a_loop_body_node_that_can_leave_the_loop_writes_into_its_own_buffer() {
    let tokens = expansion(
        r#"<ul>@for row in rows { <b>"kept"</b> <li>{{ if row.skip { continue } row.name }}</li> "end" }</ul>"#,
    );
    // `<li>` is 9 bytes at least (`<li></li>`).
    assert!(
        tokens.contains(
            r#"{ let mut s1 = String :: with_capacity (9usize) ; s1 . push_str ("<li>") ;"#
        ),
        "{tokens}"
    );
    assert!(
        tokens.contains(r#"s1 . push_str ("</li>") ; s . push_str (& s1) ; }"#),
        "{tokens}"
    );
    // The nodes before and after it write into the document buffer.
    assert!(
        tokens.contains(r#"s . push_str ("<b>kept</b>") ;"#),
        "{tokens}"
    );
    assert!(tokens.contains(r#"s . push_str ("end") ;"#), "{tokens}");
    // Outside a loop body the same node needs no buffer of its own.
    let tokens = expansion("<li>{{ if skip { continue } name }}</li>");
    assert!(!tokens.contains("s1"), "{tokens}");
}

#[test]
fn nested_node_buffers_are_numbered_by_depth() {
    let tokens = expansion(
        "@for row in rows { <p>@for cell in { if row.empty { break } row.cells } { <i>{{ if cell.hidden { continue } cell }}</i> }</p> }",
    );
    assert!(
        tokens.contains("let mut s1 = String :: with_capacity (7usize)"),
        "{tokens}"
    );
    assert!(
        tokens.contains("let mut s2 = String :: with_capacity (7usize)"),
        "{tokens}"
    );
    assert!(tokens.contains("s1 . push_str (& s2) ;"), "{tokens}");
    assert!(tokens.contains("s . push_str (& s1) ;"), "{tokens}");
}

#[test]
fn only_continue_and_break_count_as_leaving_a_loop() {
    for (source, leaves) in [
        ("continue", true),
        ("break", true),
        ("{ if x { break } y }", true),
        ("f(|| loop { break })", true),
        ("[a, (b, { continue })]", true),
        ("return x", false),
        ("x?", false),
        ("breaking + continued", false),
        (r#""break""#, false),
        ("'a: { 1 }", false),
    ] {
        let expr: syn::Expr = match syn::parse_str(source) {
            Ok(expr) => expr,
            Err(error) => panic!("{source}: {error}"),
        };
        assert_eq!(expr_mentions_loop_exit(&expr), leaves, "{source}");
    }
}

#[test]
fn only_the_expressions_of_a_node_decide_whether_it_can_leave_a_loop() {
    for (source, leaves) in [
        ("<p>{continue}</p>", true),
        ("<a href={break}></a>", true),
        ("<a href?={continue}></a>", true),
        ("<a href=\"break\" continue></a>", false),
        ("<p>break continue</p>", false),
        ("@if { continue } { }", true),
        ("@if a { } @else if { break } { }", true),
        ("@if a { } @else { {break} }", true),
        ("@if a { <i>{continue}</i> }", true),
        ("@if a { } @else { }", false),
        ("@match { break } { _ => {} }", true),
        ("@match m { _ if { continue } => {} _ => {} }", true),
        ("@match m { _ => { {break} } }", true),
        ("@match m { _ => {} }", false),
        // `continue` in the body of an inner `@for` leaves the inner loop.
        ("@for x in { break } { }", true),
        ("@for x in xs { {continue} }", false),
    ] {
        let document = match syn::parse_str::<HtmlDocument>(source) {
            Ok(document) => document,
            Err(error) => panic!("{source}: {error}"),
        };
        let [node] = &document.nodes[..] else {
            panic!("expected one node for {source}");
        };
        assert_eq!(node.mentions_loop_exit(), leaves, "{source}");
    }
}

#[test]
fn text_written_before_code_is_flushed_first() {
    let mut writer = Writer::new(Ident::new("out", Span::call_site()), 0);
    writer.text("a");
    writer.text("b");
    writer.code(quote::quote! { f(); });
    writer.text("c");
    assert_eq!(
        writer.finish().to_string(),
        r#"out . push_str ("ab") ; f () ; out . push_str ("c") ;"#
    );
    let empty = Writer::new(Ident::new("out", Span::call_site()), 0);
    assert_eq!(empty.finish().to_string(), "");
}
