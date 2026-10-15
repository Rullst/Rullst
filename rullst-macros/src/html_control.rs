#![cfg_attr(mutants, mutants::skip)]
//! `@if`/`@else if`/`@else` and `@for` blocks inside `html!`.
//!
//! A node cannot start with `@` otherwise, so the syntax is unambiguous. The
//! condition and the iterable are parsed like Rust's own `if`/`for` heads
//! (`if let` and patterns included); bodies are `html!` nodes in braces.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::ParseStream;
use syn::{Expr, Ident, Pat, Result, Token};

use super::{HtmlNode, parse_children};

pub struct HtmlIf {
    /// The `@if` branch followed by every `@else if` branch.
    pub branches: Vec<(Expr, Vec<HtmlNode>)>,
    pub otherwise: Option<Vec<HtmlNode>>,
}

pub struct HtmlFor {
    pub pattern: Pat,
    pub iterable: Expr,
    pub body: Vec<HtmlNode>,
}

pub(super) fn parse_control(input: ParseStream) -> Result<HtmlNode> {
    let at = input.parse::<Token![@]>()?;
    if input.peek(Token![if]) {
        parse_if(input).map(|node| HtmlNode::If(Box::new(node)))
    } else if input.peek(Token![for]) {
        parse_for(input).map(|node| HtmlNode::For(Box::new(node)))
    } else if input.peek(Token![else]) {
        Err(syn::Error::new(
            at.span,
            "`@else` must directly follow the body of an `@if` or `@else if` block",
        ))
    } else {
        Err(syn::Error::new(
            at.span,
            "html! supports `@if`, `@else` and `@for`; for `match` and other control flow, \
             interpolate a Rust expression that returns `Markup`, such as \
             `{match state { State::A => html! { .. }, State::B => html! { .. } }}`",
        ))
    }
}

pub(super) fn else_without_at(input: ParseStream) -> syn::Error {
    input.error("write `@else` after an `@if` block in html!")
}

fn parse_if(input: ParseStream) -> Result<HtmlIf> {
    let mut branches = Vec::new();
    let mut otherwise = None;
    loop {
        input.parse::<Token![if]>()?;
        let condition = Expr::parse_without_eager_brace(input)?;
        branches.push((condition, parse_body(input)?));
        if !(input.peek(Token![@]) && input.peek2(Token![else])) {
            break;
        }
        input.parse::<Token![@]>()?;
        input.parse::<Token![else]>()?;
        if !input.peek(Token![if]) {
            otherwise = Some(parse_body(input)?);
            break;
        }
    }
    Ok(HtmlIf {
        branches,
        otherwise,
    })
}

fn parse_for(input: ParseStream) -> Result<HtmlFor> {
    input.parse::<Token![for]>()?;
    let pattern = Pat::parse_multi_with_leading_vert(input)?;
    input.parse::<Token![in]>()?;
    let iterable = Expr::parse_without_eager_brace(input)?;
    let body = parse_body(input)?;
    Ok(HtmlFor {
        pattern,
        iterable,
        body,
    })
}

/// Parses `{ nodes }`, the body of a control block.
fn parse_body(input: ParseStream) -> Result<Vec<HtmlNode>> {
    let content;
    syn::braced!(content in input);
    let nodes = parse_children(&content)?;
    if !content.is_empty() {
        return Err(content.error("unexpected closing tag in an `@if`/`@for` body"));
    }
    Ok(nodes)
}

/// Statements appending each node of `body` to `buffer`.
fn push_nodes(buffer: &Ident, body: &[HtmlNode]) -> TokenStream {
    let nodes = body.iter().map(HtmlNode::to_tokens);
    quote! { #( #buffer.push_str(&#nodes); )* }
}

impl HtmlIf {
    /// A block evaluating to the `String` of the branch that matched.
    pub fn to_tokens(&self) -> TokenStream {
        let buffer = Ident::new("s", Span::mixed_site());
        let mut chain = TokenStream::new();
        for (index, (condition, body)) in self.branches.iter().enumerate() {
            let pushes = push_nodes(&buffer, body);
            if index > 0 {
                chain.extend(quote! { else });
            }
            chain.extend(quote! { if #condition { #pushes } });
        }
        if let Some(body) = &self.otherwise {
            let pushes = push_nodes(&buffer, body);
            chain.extend(quote! { else { #pushes } });
        }
        quote! {
            {
                let mut #buffer = String::new();
                #chain
                #buffer
            }
        }
    }
}

impl HtmlFor {
    /// A block evaluating to the `String` of every iteration.
    pub fn to_tokens(&self) -> TokenStream {
        let buffer = Ident::new("s", Span::mixed_site());
        let pushes = push_nodes(&buffer, &self.body);
        let pattern = &self.pattern;
        let iterable = &self.iterable;
        quote! {
            {
                let mut #buffer = String::new();
                for #pattern in #iterable { #pushes }
                #buffer
            }
        }
    }
}
