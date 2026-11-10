#![cfg_attr(mutants, mutants::skip)]
//! Unquoted text inside `html!`: `<p>Hello, world!</p>`.
//!
//! Rust hands a macro tokens, not characters, so the text is rebuilt from
//! the tokens and the source positions between them:
//!
//! - words, numbers and punctuation written together stay together, and any
//!   whitespace (or comment) between two of them becomes one space;
//! - between unquoted text and a neighbouring node (an element, `{…}`, a
//!   string literal, an `@` block), whitespace in the source becomes one
//!   space and none stays none;
//! - whitespace at the start or end of an element's content, of an `@` block
//!   body, of parentheses and of the whole input is dropped;
//! - `(…)` and `[…]` are text whose content follows the same rules, so
//!   `({count})` writes the count in parentheses.
//!
//! Whitespace between two nodes that are not unquoted text is ignored, as it
//! always was. Without source positions (outside a compiler expansion, or
//! for tokens another macro generated with one span) every gap counts as
//! whitespace. `<`, `>`, `{`, `}` and `@` are `html!` syntax, quotes and
//! character literals are Rust literals and `//` starts a comment, so text
//! that contains them is written as a string literal, which keeps every
//! character and space exactly.

use proc_macro2::Span;
use syn::buffer::Cursor;
use syn::ext::IdentExt;
use syn::parse::ParseStream;
use syn::{Ident, Lit, LitStr, Result, Token, token};

use super::HtmlNode;

/// How two consecutive tokens were written in the source.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Gap {
    /// Without anything between them, such as `Hello` and `,` in `Hello,`.
    None,
    /// With whitespace or a comment between them.
    Space,
    /// Unknown: the tokens have no distinct source positions, as when another
    /// macro generated the `html!` input with one span for every token.
    Unknown,
}

/// A line and column of the source.
type Position = (usize, usize);

/// Where `span` starts and ends, from the compiler (`Span::start`/`end`,
/// stable since Rust 1.88). Outside a macro expansion, as in this crate's
/// unit tests, there is no compiler span and the position is unknown.
fn location(span: Span) -> Option<(Position, Position)> {
    if !proc_macro::is_available() {
        return None;
    }
    let span = span.unwrap();
    let (start, end) = (span.start(), span.end());
    Some(((start.line(), start.column()), (end.line(), end.column())))
}

/// How `next` follows `previous`, from their source positions.
pub(super) fn gap(previous: Span, next: Span) -> Gap {
    let (Some((start, end)), Some((next_start, _))) = (location(previous), location(next)) else {
        return Gap::Unknown;
    };
    if next_start <= start {
        Gap::Unknown
    } else if next_start == end {
        Gap::None
    } else {
        Gap::Space
    }
}

/// One child of an element (or of a block body, parentheses or the input)
/// before the spaces between children are decided.
pub(super) struct Child {
    pub(super) nodes: Vec<HtmlNode>,
    /// Unquoted text or a text group, whose neighbours' whitespace counts.
    pub(super) text: bool,
    /// Whether whitespace (or nothing known) separates it from the token
    /// before it.
    pub(super) spaced: bool,
}

impl Child {
    pub(super) fn node(node: HtmlNode, spaced: bool) -> Self {
        Self {
            nodes: vec![node],
            text: false,
            spaced,
        }
    }
}

/// Whether whitespace separates the next token of `input` from the one
/// before it. Only meaningful after the first child.
pub(super) fn spaced_before(input: ParseStream) -> bool {
    gap(input.cursor().prev_span(), input.span()) != Gap::None
}

/// Joins `children`, writing one space where unquoted text meets a
/// neighbour across whitespace, and merges adjacent text.
pub(super) fn join(children: Vec<Child>) -> Vec<HtmlNode> {
    let mut nodes: Vec<HtmlNode> = Vec::new();
    let mut previous_is_text = false;
    for (index, child) in children.into_iter().enumerate() {
        if index > 0 && child.spaced && (previous_is_text || child.text) {
            push_text(&mut nodes, " ", Span::call_site());
        }
        previous_is_text = child.text;
        for node in child.nodes {
            match node {
                HtmlNode::Text(text) => push_text(&mut nodes, &text.value(), text.span()),
                other => nodes.push(other),
            }
        }
    }
    nodes
}

fn push_text(nodes: &mut Vec<HtmlNode>, text: &str, span: Span) {
    if let Some(HtmlNode::Text(last)) = nodes.last_mut() {
        *last = LitStr::new(&format!("{}{text}", last.value()), last.span());
    } else {
        nodes.push(HtmlNode::Text(LitStr::new(text, span)));
    }
}

/// `lit` without a suffix: Rust reads `"quoted"text` as one string literal
/// with the suffix `text`, which would otherwise be dropped silently.
pub(super) fn unsuffixed(lit: LitStr) -> Result<LitStr> {
    if lit.suffix().is_empty() {
        Ok(lit)
    } else {
        Err(syn::Error::new(
            lit.span(),
            format!(
                "`{}` directly after a string literal is a Rust literal suffix, not text; \
                 put a space before it or write it inside the string literal",
                lit.suffix()
            ),
        ))
    }
}

/// Whether the next token can begin unquoted text.
pub(super) fn starts_text(input: ParseStream) -> bool {
    !(input.peek(Token![<])
        || input.peek(Token![@])
        || input.peek(token::Brace)
        || input.peek(LitStr))
}

/// Parses `(…)` or `[…]` as text around its content.
pub(super) fn parse_group(input: ParseStream) -> Result<Child> {
    let content;
    let (open, close) = if input.peek(token::Paren) {
        syn::parenthesized!(content in input);
        ("(", ")")
    } else {
        syn::bracketed!(content in input);
        ("[", "]")
    };
    let inner = super::parse_children(&content)?;
    if !content.is_empty() {
        return Err(content.error(format!(
            "unexpected closing tag inside `{open}…{close}` in html!"
        )));
    }
    let mut nodes = vec![HtmlNode::Text(LitStr::new(open, Span::call_site()))];
    nodes.extend(inner);
    nodes.push(HtmlNode::Text(LitStr::new(close, Span::call_site())));
    Ok(Child {
        nodes,
        text: true,
        spaced: false,
    })
}

/// One token of unquoted text: what it writes, and where it starts and ends.
struct Word {
    text: String,
    start: Span,
    end: Span,
}

/// Parses words, numbers and punctuation up to the next node.
pub(super) fn parse_run(input: ParseStream) -> Result<Child> {
    let mut text = String::new();
    let mut first: Option<Word> = None;
    let mut previous: Option<Word> = None;
    while !input.is_empty()
        && starts_text(input)
        && !input.peek(token::Paren)
        && !input.peek(token::Bracket)
    {
        let word = input.step(|cursor| next_word(*cursor))?;
        if let Some(previous) = &previous {
            reject_url(previous, &word, &text)?;
            if gap(previous.end, word.start) != Gap::None {
                text.push(' ');
            }
        }
        text.push_str(&word.text);
        if word.text == "#" {
            reject_doc_comment(input, word.start)?;
        }
        if first.is_none() {
            first = Some(Word {
                text: word.text.clone(),
                start: word.start,
                end: word.end,
            });
        }
        previous = Some(word);
    }
    let Some(first) = first else {
        return Err(input.error("expected an element, `{…}`, text or a string literal"));
    };
    reject_control_without_at(input, &first)?;
    if let Some(last) = &previous {
        let spaced = input.is_empty() || gap(last.end, input.span()) != Gap::None;
        reject_url_end(last, &text, spaced)?;
    }
    Ok(Child {
        nodes: vec![HtmlNode::Text(LitStr::new(&text, first.start))],
        text: true,
        spaced: false,
    })
}

fn next_word(cursor: Cursor<'_>) -> Result<(Word, Cursor<'_>)> {
    if let Some((ident, rest)) = cursor.ident() {
        let word = Word {
            text: ident.to_string(),
            start: ident.span(),
            end: ident.span(),
        };
        return Ok((word, rest));
    }
    if let Some((lifetime, rest)) = cursor.lifetime() {
        // An apostrophe inside a word: `don't` is `don` and `'t`.
        let word = Word {
            text: lifetime.to_string(),
            start: lifetime.apostrophe,
            end: lifetime.ident.span(),
        };
        return Ok((word, rest));
    }
    if let Some((punct, rest)) = cursor.punct() {
        if punct.as_char() == '>' {
            return Err(syn::Error::new(
                punct.span(),
                "`>` is tag syntax in html!; write text that contains it as a string \
                 literal (`\"a > b\"`) or as `&gt;`",
            ));
        }
        let word = Word {
            text: punct.as_char().to_string(),
            start: punct.span(),
            end: punct.span(),
        };
        return Ok((word, rest));
    }
    if let Some((literal, rest)) = cursor.literal() {
        let span = literal.span();
        return match Lit::new(literal) {
            Lit::Int(number) => Ok((word(number.to_string(), span), rest)),
            Lit::Float(number) => Ok((word(number.to_string(), span), rest)),
            _ => Err(syn::Error::new(
                span,
                "character and byte literals are not text in html!; write the text as a \
                 string literal, such as `\"it's 'quoted'\"`",
            )),
        };
    }
    Err(syn::Error::new(
        cursor.span(),
        "expected an element, `{…}`, text or a string literal",
    ))
}

fn word(text: String, span: Span) -> Word {
    Word {
        text,
        start: span,
        end: span,
    }
}

/// `https:` followed by a gap is a URL whose `//…` the compiler dropped as a
/// comment.
fn reject_url(previous: &Word, next: &Word, text: &str) -> Result<()> {
    if previous.text == ":" && ends_with_scheme(text) && gap(previous.end, next.start) != Gap::None
    {
        return Err(url_error(previous.end));
    }
    Ok(())
}

fn reject_url_end(last: &Word, text: &str, spaced: bool) -> Result<()> {
    if spaced && last.text == ":" && ends_with_scheme(text) {
        return Err(url_error(last.end));
    }
    Ok(())
}

fn ends_with_scheme(text: &str) -> bool {
    let before = text.strip_suffix(':').unwrap_or(text).trim_end();
    let scheme = before
        .rsplit(|c: char| !c.is_ascii_alphanumeric())
        .next()
        .unwrap_or("");
    ["http", "https", "ftp", "ws", "wss"]
        .iter()
        .any(|known| scheme.eq_ignore_ascii_case(known))
}

fn url_error(span: Span) -> syn::Error {
    syn::Error::new(
        span,
        "`//` starts a Rust comment, so the rest of this URL is not text in html!; write \
         the URL as a string literal, such as `\"https://example.com\"`",
    )
}

/// `///` and `//!` comments reach a macro as `#[doc = "…"]`, which must not
/// become visible text.
fn reject_doc_comment(input: ParseStream, at: Span) -> Result<()> {
    let attribute = input.fork();
    if attribute.peek(Token![!]) {
        let _ = attribute.parse::<Token![!]>();
    }
    if !attribute.peek(token::Bracket) {
        return Ok(());
    }
    let content;
    syn::bracketed!(content in attribute);
    if content.peek(Ident::peek_any) && content.call(Ident::parse_any)? == "doc" {
        return Err(syn::Error::new(
            at,
            "doc comments (`///`, `//!`) are not text in html!; write a `//` comment, \
             or put the text in a string literal",
        ));
    }
    Ok(())
}

/// `if cond { <tag> }` without `@` is not text: point to the control form.
fn reject_control_without_at(input: ParseStream, first: &Word) -> Result<()> {
    if !["if", "for", "match"].contains(&first.text.as_str()) || !input.peek(token::Brace) {
        return Ok(());
    }
    let block = input.fork();
    let content;
    syn::braced!(content in block);
    if content.peek(Token![<]) {
        return Err(syn::Error::new(
            first.start,
            format!(
                "write `@{0}` for a control block in html!; unquoted `{0}` followed by \
                 `{{ <tag> … }}` would be text",
                first.text
            ),
        ));
    }
    Ok(())
}
