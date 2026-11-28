//! `@if`/`@else if`/`@else`, `@for` and `@match` blocks inside `html!`.
//!
//! A node cannot start with `@` otherwise, so the syntax is unambiguous. The
//! condition, the iterable, the scrutinee, patterns and guards are parsed
//! like Rust's own `if`/`for`/`match` (`if let` included); bodies are `html!`
//! nodes in braces.

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::ParseStream;
use syn::{Expr, Pat, Result, Token, token};

use super::write::{Helpers, Writer, expr_mentions_loop_exit, write_loop_body};
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

/// `@match scrutinee { pattern if guard => { nodes } … }`.
pub struct HtmlMatch {
    /// Keeps the caller's span, so exhaustiveness errors point at the source.
    pub match_token: Token![match],
    pub scrutinee: Expr,
    pub arms: Vec<HtmlArm>,
}

pub struct HtmlArm {
    pub pattern: Pat,
    pub guard: Option<Expr>,
    pub body: Vec<HtmlNode>,
}

pub(super) fn parse_control(input: ParseStream) -> Result<HtmlNode> {
    let at = input.parse::<Token![@]>()?;
    if input.peek(Token![if]) {
        parse_if(input).map(|node| HtmlNode::If(Box::new(node)))
    } else if input.peek(Token![for]) {
        parse_for(input).map(|node| HtmlNode::For(Box::new(node)))
    } else if input.peek(Token![match]) {
        parse_match(input).map(|node| HtmlNode::Match(Box::new(node)))
    } else if input.peek(Token![else]) {
        Err(syn::Error::new(
            at.span,
            "`@else` must directly follow the body of an `@if` or `@else if` block",
        ))
    } else {
        Err(syn::Error::new(
            at.span,
            "html! supports `@if`, `@else`, `@for` and `@match`; for other control flow, \
             interpolate a Rust expression that returns `Markup`, such as \
             `{rows.iter().map(|row| html! { .. }).collect::<Markup>()}`; write a literal `@` \
             (an e-mail address or a handle) in a string literal",
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

fn parse_match(input: ParseStream) -> Result<HtmlMatch> {
    let match_token = input.parse::<Token![match]>()?;
    let scrutinee = Expr::parse_without_eager_brace(input)?;
    let content;
    syn::braced!(content in input);
    let mut arms = Vec::new();
    while !content.is_empty() {
        let pattern = Pat::parse_multi_with_leading_vert(&content)?;
        let guard = if content.peek(Token![if]) {
            content.parse::<Token![if]>()?;
            Some(content.parse::<Expr>()?)
        } else {
            None
        };
        content.parse::<Token![=>]>()?;
        if !content.peek(token::Brace) {
            return Err(content.error(
                "write the nodes of an `@match` arm in braces: `pattern => { <p>\"text\"</p> }`",
            ));
        }
        let body = parse_body(&content)?;
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
        arms.push(HtmlArm {
            pattern,
            guard,
            body,
        });
    }
    Ok(HtmlMatch {
        match_token,
        scrutinee,
        arms,
    })
}

/// Parses `{ nodes }`, the body of a control block.
fn parse_body(input: ParseStream) -> Result<Vec<HtmlNode>> {
    let content;
    syn::braced!(content in input);
    let nodes = parse_children(&content)?;
    if !content.is_empty() {
        return Err(content.error("unexpected closing tag in an `@if`, `@for` or `@match` body"));
    }
    Ok(nodes)
}

/// The statements writing the nodes of a block body into `out`'s buffer.
fn write_body(body: &[HtmlNode], out: &Writer, helpers: &mut Helpers) -> TokenStream {
    let mut writer = out.body();
    for node in body {
        node.write(&mut writer, helpers);
    }
    writer.finish()
}

fn any_mentions_loop_exit(body: &[HtmlNode]) -> bool {
    body.iter().any(HtmlNode::mentions_loop_exit)
}

impl HtmlIf {
    /// Appends the `if` chain writing the branch that matched.
    pub(super) fn write(&self, out: &mut Writer, helpers: &mut Helpers) {
        let mut chain = TokenStream::new();
        for (index, (condition, body)) in self.branches.iter().enumerate() {
            let writes = write_body(body, out, helpers);
            if index > 0 {
                chain.extend(quote! { else });
            }
            chain.extend(quote! { if #condition { #writes } });
        }
        if let Some(body) = &self.otherwise {
            let writes = write_body(body, out, helpers);
            chain.extend(quote! { else { #writes } });
        }
        out.code(chain);
    }

    pub(super) fn mentions_loop_exit(&self) -> bool {
        self.branches.iter().any(|(condition, body)| {
            expr_mentions_loop_exit(condition) || any_mentions_loop_exit(body)
        }) || self
            .otherwise
            .as_deref()
            .is_some_and(any_mentions_loop_exit)
    }
}

impl HtmlFor {
    /// Appends the loop writing every iteration.
    pub(super) fn write(&self, out: &mut Writer, helpers: &mut Helpers) {
        let mut body = out.body();
        write_loop_body(&self.body, &mut body, helpers);
        let writes = body.finish();
        let pattern = &self.pattern;
        let iterable = &self.iterable;
        out.code(quote! { for #pattern in #iterable { #writes } });
    }
}

impl HtmlMatch {
    /// Appends the `match` writing the arm that matched. Rust checks the
    /// patterns, guards and exhaustiveness as for its own `match`.
    pub(super) fn write(&self, out: &mut Writer, helpers: &mut Helpers) {
        let match_token = &self.match_token;
        let scrutinee = &self.scrutinee;
        let arms: Vec<TokenStream> = self
            .arms
            .iter()
            .map(|arm| {
                let pattern = &arm.pattern;
                let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
                let writes = write_body(&arm.body, out, helpers);
                quote! { #pattern #guard => { #writes } }
            })
            .collect();
        out.code(quote! { #match_token #scrutinee { #(#arms)* } });
    }

    pub(super) fn mentions_loop_exit(&self) -> bool {
        expr_mentions_loop_exit(&self.scrutinee)
            || self.arms.iter().any(|arm| {
                arm.guard.as_ref().is_some_and(expr_mentions_loop_exit)
                    || any_mentions_loop_exit(&arm.body)
            })
    }
}
